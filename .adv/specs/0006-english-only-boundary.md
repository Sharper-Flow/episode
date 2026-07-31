# rq-episode-english-only-boundary01

## Statement

Given non-English memory input, when English-only support remains the chosen boundary, then repository documentation states that boundary.

## Rationale

The v0 embedding model is the local fastembed BGE-large model, which is English-centric. Without an explicit boundary, operators may assume multilingual support and store poorly embedded memories.

## Authority

- `src/embed.rs` configures the local fastembed BGE model.
- `README.md` and `project.md` declare the English-only boundary.
- `EPISODE_EMBED_BACKEND` accepts only `local` in v0.

## Verification

- `README.md` and `project.md` explicitly state that v0 supports English input only.

## Constraints

- Multilingual embedding support and embedding-model replacement are out of scope for this change.
