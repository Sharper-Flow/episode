# Cross-namespace memory id collision, fixed

## What this delivers

Episode stores each project's lessons in its own namespace. The lesson's id, however, was globally unique across the whole database. Those two rules disagreed: ingested lesson ids are raw per-project ADV ids like `pw-1`, so the moment a second project was watched, both projects held a row called `pw-1` and only one could exist.

The second project's reconcile silently stole the first project's row: content, metadata, and namespace all replaced in place, with no error and no log line. The first project's memory simply vanished, and whatever subsystem read it next would take the blame.

The fix is one structural change: a row's identity is now the pair (namespace, id) — the primary key — instead of the id alone. Two projects can both hold `pw-1` because they are different rows. The steal is impossible by construction rather than guarded against.

## How it was found

The defect reproduced by accident during unrelated work: two new tests in the promotion-state change each seeded `pw-1` under different namespaces and collided, presenting as a promotion bug. It was not one. It was routed to its own change with the reproduction attached, and that reproduction became this change's regression test.

## Confidence

The regression test was proven load-bearing on isolated databases rather than assumed: against the old schema it fails with exactly the steal (one row where two belong), and against the new schema both rows survive. The migration was also exercised from an empty database, not just an upgraded one.

An independent review confirmed the design before implementation and a second independent review checked the result against the acceptance criteria. The second review tightened three tests that were weaker than their claims, including retiring the last test workaround that existed only because of this defect.

One defect in my own regression test surfaced mid-run and was fixed rather than papered over: it initially asserted against every row in the shared test database instead of the run's own namespaces — the same global-uniqueness assumption this change removes, applied by habit.

## Current state and impact

Nothing was harmed: the episode service has been switched off since 19 August 2026 and the store holds zero rows. The defect would have gone live on the first day a second project was watched after restoration. That restoration is now unblocked by this fix.

## Behavior note

Recall spanning several namespaces may now return hits carrying the same id from different namespaces. That is intended: they are distinct memories that merely share a raw source id. Every addressing surface (promote, forget) already required the namespace beside the id, so no caller contract changed.
