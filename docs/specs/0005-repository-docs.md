# rq-episode-repository-docs01

## Statement

Given repository setup or operations, when a contributor follows maintained documentation, then all referenced config and specification paths exist.

## Rationale

Operational documentation is only trustworthy if every path it points to exists in the repository. Drift between README, project context, and specs makes onboarding and maintenance error-prone.

## Authority

- `README.md` is the contributor entry point.
- `project.md` is the ADV project context file declared in `project.json`.
- `.adv/specs/` and `docs/specs/` are declared in `project.json`.

## Verification

- Static path validation confirms `README.md`, `project.md`, `.env.example`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/release.md`, `.adv/specs/`, and `docs/specs/` exist.
- Every markdown link and code path in the operational docs resolves to an existing file or directory.

## Constraints

- `.adv/archive/` archives are immutable; prior release documentation must be preserved.
