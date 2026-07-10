//! Bounded priority embedding scheduler (AC4 / AC5 / AC6, C2 / C4 / C5).
//!
//! Owns the single local [`Embedder`] and multiplexes two typed job classes
//! through bounded channels:
//!
//! - **interactive** — one text, used by `recall` / `remember`.
//! - **ingestion** — one bounded batch (≤ [`INGEST_BATCH_MAX`] texts).
//!
//! A single worker task selects jobs with *biased* polling: a queued
//! interactive job is always chosen before another ingestion batch, so a
//! `recall` that arrives during ingestion is served before the next batch
//! begins (AC5). Each selected job runs the existing synchronous embedder
//! inside `spawn_blocking`; the worker **awaits** that job before selecting
//! another, so there is never more than one active inference and the one-model
//! memory budget is preserved (C4 — no second model is ever created).
//!
//! Shutdown is cooperative (AC6 / C5): a `tokio::sync::watch` signal is polled
//! between jobs. When it fires, the worker stops selecting new work and exits
//! after the active job (if any) returns. Active `spawn_blocking` inference is
//! never aborted — Tokio cannot cancel it, so we simply do not start the next
//! job.
//!
//! Testability seam: an injected [`ExecRecorder`] observes the kind of every
//! selected job. Production uses [`NoopRecorder`]; tests inject a recorder to
//! assert selection order deterministically.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::embed::Embedder;
use crate::types::EMBEDDING_DIM;

/// Maximum number of texts in a single ingestion batch (AC4). Ingestion
/// partitions eligible memories into chunks of at most this size, so 65+
/// eligible memories produce multiple bounded batches instead of one
/// unbounded embedding call.
pub const INGEST_BATCH_MAX: usize = 64;

/// The kind of job the scheduler selected, reported to the [`ExecRecorder`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobKind {
    /// One-text `recall` / `remember` request.
    Interactive,
    /// One bounded ingestion batch.
    Ingest,
}

/// Observes scheduler selection order. Injected so tests can prove AC5/AC6
/// deterministically without timing the real model.
pub trait ExecRecorder: Send + Sync {
    /// Called once per selected job, in selection order, before the job runs.
    fn record(&self, kind: JobKind);
}

/// Production recorder: records nothing.
#[derive(Default, Clone, Copy, Debug)]
pub struct NoopRecorder;

impl ExecRecorder for NoopRecorder {
    fn record(&self, _kind: JobKind) {}
}

/// Scheduler construction parameters.
#[derive(Clone, Copy, Debug)]
pub struct SchedulerConfig {
    /// Bounded capacity of each job channel (backpressure).
    pub capacity: usize,
    /// Maximum ingestion batch size accepted by the handle (≤ [`INGEST_BATCH_MAX`]).
    pub ingest_max: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            capacity: 64,
            ingest_max: INGEST_BATCH_MAX,
        }
    }
}

struct InteractiveJob {
    text: String,
    reply: oneshot::Sender<Result<Vec<f32>>>,
}

struct IngestJob {
    texts: Vec<String>,
    reply: oneshot::Sender<Result<Vec<Vec<f32>>>>,
}

enum Job {
    Interactive(InteractiveJob),
    Ingest(IngestJob),
}

impl Job {
    fn kind(&self) -> JobKind {
        match self {
            Job::Interactive(_) => JobKind::Interactive,
            Job::Ingest(_) => JobKind::Ingest,
        }
    }
}

/// Cheap cloneable client handle to the scheduler. Holds the bounded senders;
/// the embedder itself is owned exclusively by the worker task (C4).
#[derive(Clone)]
pub struct SchedulerHandle {
    interactive_tx: mpsc::Sender<InteractiveJob>,
    ingest_tx: mpsc::Sender<IngestJob>,
    ingest_max: usize,
}

impl SchedulerHandle {
    /// Embed a single text (interactive path). Routes through the scheduler so
    /// it is prioritized ahead of queued ingestion batches (AC5).
    pub async fn embed_one(&self, text: String) -> Result<Vec<f32>> {
        let (tx, rx) = oneshot::channel();
        self.interactive_tx
            .send(InteractiveJob { text, reply: tx })
            .await
            .map_err(|_| anyhow::anyhow!("embedding scheduler is closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("embedding scheduler dropped the request"))?
    }

    /// Embed one bounded ingestion batch (AC4).
    ///
    /// - An **empty** input returns an empty result without invoking the
    ///   embedder or scheduling a job (AC: empty batches do not embed).
    /// - An input larger than `ingest_max` is rejected with a descriptive
    ///   error and is never scheduled.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        anyhow::ensure!(
            texts.len() <= self.ingest_max,
            "ingestion batch of {} exceeds maximum of {}",
            texts.len(),
            self.ingest_max
        );
        let (tx, rx) = oneshot::channel();
        self.ingest_tx
            .send(IngestJob { texts, reply: tx })
            .await
            .map_err(|_| anyhow::anyhow!("embedding scheduler is closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("embedding scheduler dropped the request"))?
    }
}

/// Validate a batch embedding result against the batch contract (design §3):
/// output length must equal the input length and every vector must be exactly
/// [`EMBEDDING_DIM`]. A mismatch fails the batch with structured context so the
/// caller can skip persistence and leave the items eligible for the next
/// reconcile.
pub fn validate_batch_output(input_len: usize, output: &[Vec<f32>]) -> Result<()> {
    anyhow::ensure!(
        output.len() == input_len,
        "embedding batch length mismatch: expected {input_len} vectors, got {}",
        output.len()
    );
    for (i, v) in output.iter().enumerate() {
        anyhow::ensure!(
            v.len() == EMBEDDING_DIM,
            "embedding dimension mismatch at index {i}: expected {EMBEDDING_DIM}, got {}",
            v.len()
        );
    }
    Ok(())
}

/// Start the scheduler worker. Returns a cloneable handle and the owned worker
/// [`JoinHandle`]; the caller retains the handle and joins the worker during
/// cooperative shutdown (AC6). The embedder is moved into the worker and is
/// never duplicated.
pub fn start(
    embedder: Arc<dyn Embedder>,
    config: SchedulerConfig,
    recorder: Arc<dyn ExecRecorder>,
    shutdown: watch::Receiver<bool>,
) -> (SchedulerHandle, JoinHandle<()>) {
    let (interactive_tx, interactive_rx) = mpsc::channel(config.capacity);
    let (ingest_tx, ingest_rx) = mpsc::channel(config.capacity);
    let handle = SchedulerHandle {
        interactive_tx,
        ingest_tx,
        ingest_max: config.ingest_max,
    };
    let worker = tokio::spawn(run_worker(
        embedder,
        interactive_rx,
        ingest_rx,
        recorder,
        shutdown,
    ));
    (handle, worker)
}

fn join_to_result<T>(res: std::result::Result<Result<T>, tokio::task::JoinError>) -> Result<T> {
    match res {
        Ok(r) => r,
        Err(e) => Err(anyhow::anyhow!("embedding task failed: {e}")),
    }
}

async fn run_worker(
    embedder: Arc<dyn Embedder>,
    interactive_rx: mpsc::Receiver<InteractiveJob>,
    ingest_rx: mpsc::Receiver<IngestJob>,
    recorder: Arc<dyn ExecRecorder>,
    mut shutdown: watch::Receiver<bool>,
) {
    // Wrap each receiver so a closed channel can be removed from the `select!`.
    // A closed `mpsc::Receiver` returns `None` from `recv()` *immediately* and is
    // therefore always ready; if it were left in the biased `select!` it would
    // either spin or — as previously — terminate the loop and abandon the other
    // queue. Disabling the arm once its channel closes keeps the surviving queue
    // fully serviced. The worker exits only on shutdown or when BOTH queues are
    // closed.
    let mut interactive_rx = Some(interactive_rx);
    let mut ingest_rx = Some(ingest_rx);

    enum Sel {
        Shutdown,
        InteractiveClosed,
        IngestClosed,
        Job(Job),
    }

    loop {
        // Cooperative stop: if shutdown was already requested, exit before
        // selecting any new work (AC6).
        if *shutdown.borrow() {
            break;
        }
        // Nothing left to service: both sender classes are gone.
        if interactive_rx.is_none() && ingest_rx.is_none() {
            break;
        }

        // Biased polling order: shutdown wins, then interactive, then ingest.
        // When not shutting down this means a queued interactive job is always
        // selected before another ingestion batch (AC5). Listing shutdown first
        // guarantees that once cancellation is signaled, no further batch is
        // started even if ingest jobs are already queued. Each channel arm is
        // guarded by `if <rx>.is_some()` so a closed channel is never polled.
        let sel = tokio::select! {
            biased;
            _ = shutdown.changed() => Sel::Shutdown,
            j = async { interactive_rx.as_mut().unwrap().recv().await }, if interactive_rx.is_some() => match j {
                Some(j) => Sel::Job(Job::Interactive(j)),
                None => Sel::InteractiveClosed,
            },
            j = async { ingest_rx.as_mut().unwrap().recv().await }, if ingest_rx.is_some() => match j {
                Some(j) => Sel::Job(Job::Ingest(j)),
                None => Sel::IngestClosed,
            },
        };

        let job = match sel {
            Sel::Shutdown => break,
            Sel::InteractiveClosed => {
                interactive_rx = None;
                continue;
            }
            Sel::IngestClosed => {
                ingest_rx = None;
                continue;
            }
            Sel::Job(job) => job,
        };

        let kind = job.kind();
        recorder.record(kind);

        // One active job at a time: run the synchronous embedder in a blocking
        // thread and await completion before looping. This is what preserves
        // the one-model memory budget (C4) and makes fairness deterministic.
        match job {
            Job::Interactive(j) => {
                let emb = embedder.clone();
                let text = j.text;
                let res = tokio::task::spawn_blocking(move || emb.embed_one(&text)).await;
                let _ = j.reply.send(join_to_result(res));
            }
            Job::Ingest(j) => {
                let emb = embedder.clone();
                let texts = j.texts;
                let res = tokio::task::spawn_blocking(move || emb.embed(&texts)).await;
                let _ = j.reply.send(join_to_result(res));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    // ---- test doubles -----------------------------------------------------

    /// Records selected job kinds in order.
    #[derive(Default)]
    struct VecRecorder(Mutex<Vec<JobKind>>);

    impl VecRecorder {
        fn order(&self) -> Vec<JobKind> {
            self.0.lock().unwrap().clone()
        }
    }

    impl ExecRecorder for VecRecorder {
        fn record(&self, kind: JobKind) {
            self.0.lock().unwrap().push(kind);
        }
    }

    /// Returns correctly-dimensioned zero vectors and counts `embed` calls.
    struct CountingEmbedder {
        dim: usize,
        calls: AtomicUsize,
    }

    impl CountingEmbedder {
        fn new(dim: usize) -> Self {
            Self {
                dim,
                calls: AtomicUsize::new(0),
            }
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl Embedder for CountingEmbedder {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(texts.iter().map(|_| vec![0.0f32; self.dim]).collect())
        }
    }

    /// Blocks inside `embed` until the test releases it, so we can enqueue
    /// additional jobs while one is active and observe selection order. Because
    /// the worker runs only one job at a time, a single shared release channel
    /// is sufficient.
    struct BarrierEmbedder {
        started: std::sync::mpsc::Sender<()>,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Embedder for BarrierEmbedder {
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            let _ = self.started.send(());
            let rx = self.release.lock().unwrap();
            let _ = rx.recv();
            Ok(texts.iter().map(|_| vec![0.0f32; EMBEDDING_DIM]).collect())
        }
    }

    fn barrier() -> (
        Arc<dyn Embedder>,
        std::sync::mpsc::Receiver<()>,
        std::sync::mpsc::Sender<()>,
    ) {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let emb: Arc<dyn Embedder> = Arc::new(BarrierEmbedder {
            started: started_tx,
            release: Mutex::new(release_rx),
        });
        (emb, started_rx, release_tx)
    }

    // ---- shape / happy path ----------------------------------------------

    #[tokio::test]
    async fn embed_one_and_batch_return_correct_shapes() {
        let emb = Arc::new(CountingEmbedder::new(EMBEDDING_DIM));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handle, worker) = start(
            emb.clone(),
            SchedulerConfig::default(),
            Arc::new(NoopRecorder),
            shutdown_rx,
        );

        let one = handle.embed_one("hello".into()).await.expect("embed_one");
        assert_eq!(one.len(), EMBEDDING_DIM);

        let batch = handle
            .embed_batch(vec!["a".into(), "b".into(), "c".into()])
            .await
            .expect("batch");
        assert_eq!(batch.len(), 3);
        assert!(batch.iter().all(|v| v.len() == EMBEDDING_DIM));

        // One interactive embed + one batch embed == two embedder invocations.
        assert_eq!(emb.calls(), 2);

        let _ = shutdown_tx.send(true);
        let _ = worker.await;
    }

    // ---- AC4: bounded / empty batch contract ------------------------------

    #[tokio::test]
    async fn empty_batch_does_not_invoke_embedding() {
        let emb = Arc::new(CountingEmbedder::new(EMBEDDING_DIM));
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handle, worker) = start(
            emb.clone(),
            SchedulerConfig::default(),
            recorder.clone(),
            shutdown_rx,
        );

        let out = handle.embed_batch(vec![]).await.expect("empty batch ok");
        assert!(out.is_empty());
        assert_eq!(emb.calls(), 0, "empty batch must not call the embedder");
        assert!(
            recorder.order().is_empty(),
            "empty batch must not schedule a job"
        );

        let _ = shutdown_tx.send(true);
        let _ = worker.await;
    }

    #[tokio::test]
    async fn batch_over_max_is_rejected_without_embedding() {
        let emb = Arc::new(CountingEmbedder::new(EMBEDDING_DIM));
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handle, worker) = start(
            emb.clone(),
            SchedulerConfig {
                capacity: 8,
                ingest_max: INGEST_BATCH_MAX,
            },
            recorder.clone(),
            shutdown_rx,
        );

        let texts: Vec<String> = (0..(INGEST_BATCH_MAX + 1)).map(|i| i.to_string()).collect();
        let err = handle
            .embed_batch(texts)
            .await
            .expect_err("batch larger than ingest_max must fail");
        let msg = err.to_string();
        assert!(
            msg.contains(&(INGEST_BATCH_MAX + 1).to_string())
                && msg.contains(&INGEST_BATCH_MAX.to_string()),
            "error names actual and maximum sizes: {msg}"
        );
        assert_eq!(emb.calls(), 0, "rejected batch must not call the embedder");
        assert!(
            recorder.order().is_empty(),
            "rejected batch must not schedule a job"
        );

        let _ = shutdown_tx.send(true);
        let _ = worker.await;
    }

    // ---- AC5: interactive priority over next ingest batch -----------------

    // The barrier tests block the test thread on a `std::sync::mpsc` recv to
    // observe the active embedder; they need a multi-thread runtime so the
    // spawned worker keeps making progress on another thread.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn interactive_job_selected_before_next_ingest_batch() {
        let (emb, started_rx, release_tx) = barrier();
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handle, worker) = start(
            emb,
            SchedulerConfig::default(),
            recorder.clone(),
            shutdown_rx,
        );

        // Start ingest batch A and wait until it is actively embedding.
        let h = handle.clone();
        let a = tokio::spawn(async move { h.embed_batch(vec!["a".into()]).await });
        started_rx.recv().expect("batch A started");

        // While A is active, queue an interactive job and a second ingest batch.
        let h = handle.clone();
        let interactive = tokio::spawn(async move { h.embed_one("recall".into()).await });
        let h = handle.clone();
        let b = tokio::spawn(async move { h.embed_batch(vec!["b".into()]).await });

        // Release A: the scheduler must select the queued interactive job
        // before ingest batch B.
        release_tx.send(()).unwrap();
        a.await.unwrap().expect("batch A result");

        started_rx.recv().expect("second job started (interactive)");
        release_tx.send(()).unwrap();
        interactive.await.unwrap().expect("interactive result");

        started_rx.recv().expect("third job started (ingest B)");
        release_tx.send(()).unwrap();
        b.await.unwrap().expect("batch B result");

        assert_eq!(
            recorder.order(),
            vec![JobKind::Ingest, JobKind::Interactive, JobKind::Ingest],
            "interactive recall must be served before the next ingest batch"
        );

        let _ = shutdown_tx.send(true);
        let _ = worker.await;
    }

    // ---- AC6 / C5: cooperative shutdown ----------------------------------

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_prevents_next_batch_after_active_returns() {
        let (emb, started_rx, release_tx) = barrier();
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (handle, worker) = start(
            emb,
            SchedulerConfig::default(),
            recorder.clone(),
            shutdown_rx,
        );

        // Batch A is actively embedding.
        let h = handle.clone();
        let a = tokio::spawn(async move { h.embed_batch(vec!["a".into()]).await });
        started_rx.recv().expect("batch A active");

        // Queue batch B, then signal shutdown, then let A finish.
        let h = handle.clone();
        let b = tokio::spawn(async move { h.embed_batch(vec!["b".into()]).await });
        tokio::task::yield_now().await;
        let _ = shutdown_tx.send(true);
        release_tx.send(()).unwrap();
        a.await.unwrap().expect("batch A result");

        // The worker must exit after the active job returns, without ever
        // starting batch B (no JoinHandle::abort; cooperative only).
        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .expect("worker did not exit after shutdown")
            .expect("worker task panicked");

        assert_eq!(
            recorder.order(),
            vec![JobKind::Ingest],
            "only the active batch may run; no new batch starts after shutdown"
        );
        assert!(
            started_rx.try_recv().is_err(),
            "batch B must never start after shutdown"
        );
        assert!(
            b.await.unwrap().is_err(),
            "a batch not started before shutdown is dropped, not run"
        );
    }

    // Regression: a closed interactive channel must not terminate or starve the
    // ingest queue. On the buggy code, `interactive_rx.recv()` returning `None`
    // mapped straight to a loop exit, abandoning queued ingest work; the worker
    // must instead keep servicing ingest and exit only once both queues close.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn closed_interactive_does_not_terminate_ingest_queue() {
        let (emb, started_rx, release_tx) = barrier();
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (interactive_tx, interactive_rx) = mpsc::channel(8);
        let (ingest_tx, ingest_rx) = mpsc::channel(8);
        let worker = tokio::spawn(run_worker(
            emb,
            interactive_rx,
            ingest_rx,
            recorder.clone(),
            shutdown_rx,
        ));

        // Ingest job 1 becomes active and blocks inside the barrier embedder.
        let (r1_tx, r1) = oneshot::channel();
        ingest_tx
            .send(IngestJob {
                texts: vec!["a".into()],
                reply: r1_tx,
            })
            .await
            .unwrap();
        started_rx.recv().expect("ingest job 1 active");

        // Ingest job 2 queues behind job 1.
        let (r2_tx, r2) = oneshot::channel();
        ingest_tx
            .send(IngestJob {
                texts: vec!["b".into()],
                reply: r2_tx,
            })
            .await
            .unwrap();

        // Drop the ENTIRE interactive sender class while job 1 is active and
        // job 2 is queued. This must not stop the ingest queue.
        drop(interactive_tx);

        // Release job 1: the worker must continue and pick up job 2.
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), r1)
            .await
            .expect("ingest job 1 reply timed out")
            .expect("reply 1 channel")
            .expect("embed 1 ok");

        // Job 2 must start (this is what the old code abandoned) and complete.
        started_rx
            .recv()
            .expect("ingest job 2 must start after interactive closed");
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), r2)
            .await
            .expect("ingest job 2 reply timed out")
            .expect("reply 2 channel")
            .expect("embed 2 ok");

        // Both ingest jobs recorded; no interactive job ever existed.
        assert_eq!(recorder.order(), vec![JobKind::Ingest, JobKind::Ingest]);

        // Closing the remaining ingest queue lets the worker exit on its own.
        drop(ingest_tx);
        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .expect("worker did not exit once both queues closed")
            .expect("worker panicked");
        let _ = shutdown_tx.send(true);
    }

    // Mirror direction: a closed ingest channel must not terminate the
    // interactive queue.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn closed_ingest_does_not_terminate_interactive_queue() {
        let emb = Arc::new(CountingEmbedder::new(EMBEDDING_DIM));
        let recorder = Arc::new(VecRecorder::default());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let (interactive_tx, interactive_rx) = mpsc::channel(8);
        let (ingest_tx, ingest_rx) = mpsc::channel(8);
        let worker = tokio::spawn(run_worker(
            emb.clone(),
            interactive_rx,
            ingest_rx,
            recorder.clone(),
            shutdown_rx,
        ));

        // Close ingest while interactive is empty, then let the worker poll so
        // it observes the closed ingest channel.
        drop(ingest_tx);
        tokio::task::yield_now().await;

        // Interactive work that arrives AFTER ingest closed must still be
        // serviced. On the buggy code the worker has already exited, so the
        // send fails (channel closed) or the reply times out.
        let (reply_tx, reply) = oneshot::channel();
        interactive_tx
            .send(InteractiveJob {
                text: "recall".into(),
                reply: reply_tx,
            })
            .await
            .expect("interactive queue must still be open after ingest closed");
        let v = tokio::time::timeout(Duration::from_secs(5), reply)
            .await
            .expect("interactive starved after ingest closed")
            .expect("reply channel")
            .expect("embed ok");
        assert_eq!(v.len(), EMBEDDING_DIM);

        assert_eq!(recorder.order(), vec![JobKind::Interactive]);
        assert_eq!(emb.calls(), 1);

        // Close interactive -> worker exits on its own.
        drop(interactive_tx);
        tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .expect("worker did not exit once both queues closed")
            .expect("worker panicked");
        let _ = shutdown_tx.send(true);
    }

    // ---- batch output validation (design §3) -----------------------------

    #[test]
    fn validate_accepts_matching_output() {
        let out = vec![vec![0.0f32; EMBEDDING_DIM]; 3];
        assert!(validate_batch_output(3, &out).is_ok());
    }

    #[test]
    fn validate_accepts_empty_output() {
        assert!(validate_batch_output(0, &[]).is_ok());
    }

    #[test]
    fn validate_rejects_length_mismatch() {
        let out = vec![vec![0.0f32; EMBEDDING_DIM]; 2];
        let err = validate_batch_output(3, &out).expect_err("length mismatch must fail");
        let msg = err.to_string();
        assert!(
            msg.contains('3') && msg.contains('2'),
            "error names expected and got lengths: {msg}"
        );
    }

    #[test]
    fn validate_rejects_dimension_mismatch() {
        let mut out = vec![vec![0.0f32; EMBEDDING_DIM]; 2];
        out[1] = vec![0.0f32; 7];
        let err = validate_batch_output(2, &out).expect_err("dim mismatch must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("index 1")
                && msg.contains(&EMBEDDING_DIM.to_string())
                && msg.contains('7'),
            "error names index, expected dim, and got dim: {msg}"
        );
    }
}
