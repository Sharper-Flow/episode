# Promotion state for episode memories

## What this delivers

Episode is the agent memory service. It stores lessons learned during work and hands them back when they become relevant again. Concord is the separate system that holds settled, durable knowledge — the written specs and decisions a team treats as law.

Until now those two had no connection. A lesson could graduate into a Concord spec while episode carried on serving its own older copy, so an agent could receive the same guidance twice from two places that quietly disagreed, with nothing indicating which one was authoritative. Or a lesson could stay in episode forever and never make it into the durable record at all.

This change gives every memory a recorded position on that path. A memory can be flagged as a candidate for graduation, or marked as graduated with a pointer to the Concord record that now owns it. Once a memory is marked graduated, episode stops serving it by default, so the durable record speaks alone. A candidate stays visible, because it has not graduated yet and episode is still its only source.

The move can be reversed. If the Concord record it points to is later deleted, the memory can be returned to visibility. Without that, a memory could be hidden permanently with no way to bring it back.

## Two faults fixed along the way

Both were found during this work rather than reported, and both came from the same underlying cause: episode only ever looked at a lesson once, when it first arrived. Anything that changed about that lesson afterward was invisible to it.

**Withdrawn lessons kept being served.** When a lesson was retracted at the source, episode already knew not to take in a new copy — but it had no way to remove one it had already stored. A retracted lesson would keep surfacing indefinitely.

**Graduation was never noticed.** Lessons are almost always stored first and graduate later, so the graduation signal arrived after episode had stopped looking. The feature would have appeared to work while doing nothing in practice.

Both are now handled by a single addition that lets episode revisit what it already holds.

## Two more faults, found by review

The work was reviewed twice, each pass looking at something different. Both passes found a real fault in the new code, and neither fault would have been visible from the outside until it had already caused harm.

**A memory could declare itself already graduated.** The first review found an internal check that was looser than intended. Tracing it back revealed the more serious problem beneath: the new promotion marker could be set by the incoming lesson file rather than only by episode. A lesson could have arrived claiming to be graduated and disappeared from recall immediately. Episode now strips that field on the way in, so only episode can set it.

**Updating a memory silently erased its promotion state.** The second review found that saving a memory overwrote all of its bookkeeping, including its promotion state. A graduated memory would quietly return to being served, reintroducing the exact duplication this change exists to prevent. Saving now preserves the promotion state while still refreshing everything else.

The second fault is worth noting for a further reason: an earlier analysis during design had concluded it could not happen. That conclusion was correct for one thing happening at a time and wrong the moment two overlap, and it was wrong in a way that only a review looking specifically at timing would catch.

## Current state and impact

Nothing is affected today. The episode service has been switched off since 19 August 2026 and its store is empty, so none of these faults has harmed anything. All of them would have begun causing real problems the first time the service ran again after being restored, which is the argument for fixing them before that happens.

## Confidence

All nine agreed acceptance criteria are met and covered by automated tests. The full test suite passes, including tests run against a real database rather than a simulation.

The three most serious fixes were checked more strictly than usual. Each was deliberately disabled to confirm its test then failed, proving the tests genuinely detect the fault rather than passing regardless. All were re-enabled and confirmed passing.

One claim in the specification was also corrected rather than defended. It had described the protection against two people changing the same memory at once as stronger than the code actually provides. The specification now states the real behaviour.

## Related work identified, not included

A separate pre-existing fault was found and recorded for its own change (`fixCrossNamespaceMemoryId`): memories from different projects can share an internal identifier, letting one project's memory silently overwrite another's. It is unrelated to promotion, so it was documented rather than folded into this work. It reproduced during testing here, which confirms it is real rather than theoretical.
