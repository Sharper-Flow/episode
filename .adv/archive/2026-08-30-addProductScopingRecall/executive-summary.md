# Product scoping for episode recall

## What this delivers

Episode serves memories per project. Concord organizes work one level up: a Product contains several projects. An agent working inside a Product needs its memories plus the shared pool — everything the Product's projects learned, everything stored with no Product claim — while other Products' work stays out.

The filter that was supposed to do this did the opposite of half its job. It matched only rows explicitly tagged with the Product, so every untagged memory — which is all ingested predecessor wisdom and the entire global pool — silently vanished from a Product-scoped query. The mechanism existed since August 11; nothing tested an untagged row, so the gap survived three shipped changes.

The fix is one predicate: a Product scope now matches rows tagged with that Product **or** rows carrying no Product claim at all. A row claiming a different Product stays excluded, and an explicit `product: null` counts as a claim, not an absence. The query still narrows by namespace first, so projects stay separated and only the shared pool is truly shared.

Episode still derives nothing: Product tags arrive from the caller, never from episode guessing which projects belong to which Product. That boundary keeps episode free of Concord's data model, which is the standing decision about the relationship between the two.

## The probe

This change is also the instrument for a standing question between the two systems: is external product-scoping good enough, or should Concord eventually own agent memory outright? Concord resolved that question on August 30 — episode stays external — with this probe named as the trigger that could reopen it.

Because a probe that can reopen a decision deserves better than a vibe, the protocol was pre-registered on concord#46 **before** any evidence could exist: a baseline (service restored today, empty store, corrected filter semantics), a method (capture recall transcripts from real Product-scoped sessions), a minimum corpus before anyone is allowed to judge (40+ memories across 2+ projects of one Product, 10+ tagged), and a fixed rubric. The outcome lands later, on that issue, citing transcripts.

## Confidence

The failing test came first: against the old filter, the untagged rows were dropped exactly as predicted, and the assertion output shows it. An independent design validator confirmed the predicate against PostgreSQL's documented behavior and C source before implementation, and the acceptance review then caught two real gaps — no test proved the product arm composes with the other filter arms, and my index explanation overstated what the GIN index's operator class covers. Both were fixed, and the overstated wording was corrected everywhere it had been repeated, including the design document, rather than defended.

## Current state

The service was restored from merged main earlier today. This change ships the corrected semantics; the running service picks them up at the next deploy. The corpus the probe needs starts accruing from real Product work after that.
