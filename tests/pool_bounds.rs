//! AC3: SQLx pool acquisition bound.
//!
//! Proves that an unavailable database returns an acquisition error within the
//! configured acquire limit (5s), rather than hanging indefinitely. Uses only
//! localhost TCP + tokio — no real Postgres, no model download — so it runs in
//! the default `cargo test` sweep.
//!
//! Determinism: we bind a `TcpListener` on an ephemeral port but never call
//! `accept()`. The kernel still completes the TCP 3-way handshake and queues the
//! connection in the accept backlog, so the client's TCP connect succeeds while
//! the PostgreSQL startup handshake never receives a server response. The client
//! therefore hangs inside connection establishment, which is exactly what the
//! pool's `acquire_timeout` bounds. A generous outer `tokio::time::timeout` is a
//! safety net that should never fire; if it does, the acquire bound is not being
//! enforced.

use std::time::{Duration, Instant};

use episode::store::Store;

#[tokio::test]
async fn unavailable_db_errors_within_acquire_bound() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral localhost port");
    let port = listener.local_addr().expect("listener addr").port();
    let url = format!("postgres://episode:episode@127.0.0.1:{port}/episode");

    // Safety net well above the configured 5s acquire bound.
    let safety = Duration::from_secs(20);

    let start = Instant::now();
    let outcome = tokio::time::timeout(safety, Store::connect(&url, 1)).await;
    let elapsed = start.elapsed();

    // The inner `connect` must complete (with an error) before the safety net.
    // If this `expect` trips, the acquire bound is not enforcing anything.
    let result = outcome.expect(
        "Store::connect hung past the safety timeout; pool acquire_timeout is not enforced",
    );

    // Extract the error via match (avoids `Result::unwrap_err`, which would
    // require `Store: Debug`).
    let err = match result {
        Ok(_store) => panic!("unavailable database must yield an acquisition error, got Ok(_)"),
        Err(e) => e,
    };

    // It must have actually waited up to the acquire bound (>= ~3s proves this was
    // a hung handshake cut off by the bound, not an instant connection-refused),
    // and it must return well under the safety net.
    assert!(
        elapsed >= Duration::from_secs(3),
        "returned too fast ({elapsed:?}); acquire bound may not be the limiting factor"
    );
    assert!(
        elapsed < safety,
        "hit the safety timeout ({elapsed:?}); acquire bound not enforced"
    );

    // The surfaced error should read as a pool/acquisition timeout.
    let msg = format!("{:?}", err);
    assert!(
        msg.to_ascii_lowercase().contains("timed out")
            || msg.to_ascii_lowercase().contains("timeout"),
        "expected an acquisition timeout error, got: {msg}"
    );

    drop(listener);
}
