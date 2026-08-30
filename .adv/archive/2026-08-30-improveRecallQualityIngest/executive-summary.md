# Recall quality filters and a source-agnostic ingest contract

## What this delivers

Three gaps closed in how episode serves and gathers knowledge.

**Provenance filtering.** An agent could not ask "only my own notes" or "only ingested wisdom" — recall had no source filter. It does now: a closed set (manual, adv_wisdom, adv_reflection) that rejects anything else before the query runs.

**Recency filtering.** A lesson about a module removed months ago ranked exactly like one learned yesterday. Recall now accepts a maximum age in days and drops rows first captured before the cutoff. The basis is deliberately the moment knowledge was first captured — promotion bookkeeping and re-upsert races touch a different timestamp, so "how old is this knowledge" stays stable. Recency filters; it never re-ranks. The hybrid-retrieval idea that would have blended lexical and vector scores stays a recorded candidate, blocked on calibration evidence nobody has yet.

**A source-agnostic ingest contract.** Ingestion was welded to ADV's file layout — two parsers with two different shapes, called by name from the reconcile loop. Both are now implementations of one small trait: a source is anything that can parse a project root into rows-plus-retractions-plus-graduations. Adding Concord's lesson surface later (git-backed markdown with a manifest, a different shape entirely) becomes a new implementation, not a refactor. The new spec records what an implementer must know: sources report, reconcile decides; one unreadable file never blinds the loop to the others; each source owns its id space; and non-ADV sources are items-only until the store's retraction and promotion paths are deliberately widened.

## A defect the review caught

The day-count conversion could panic. The wire type accepts values up to about 4.29 billion; the database's interval function accepts about 2.1 billion; my conversion bridged the gap with an assertion that the gap would never be crossed. A caller sending the larger number would have crashed the request. The bound is now a validation error with the range in the message, and the store itself validates rather than trusting the tool layer — the assertion that remained is unreachable by construction.

## Confidence

Each sub-part carries its own tests: closed-set and validation coverage in unit tests, source and recency behavior against a real database with backdated timestamps, and the trait's aggregation and error isolation proven with stub sources — including a sabotage check confirming the isolation test fails when isolation is removed. The strongest evidence for the refactor is what did not change: every existing reconcile, promotion, and ingestion suite passes unmodified through the trait, which was the acceptance criterion.

Two independent reviews ran: the design validator caught an impossible type binding before any code existed, and the acceptance review caught the panic plus several overstated spec claims. Both were adopted with corrections recorded where the claims had been repeated.

## Current state

Episode's original epic — structured episodic memory with capture context, action states, safe recall, product scoping, promotion, and quality controls — is complete with this change. The service is live, the corpus is accruing, and the product-scoping probe on concord#46 is collecting evidence under its pre-registered rubric.
