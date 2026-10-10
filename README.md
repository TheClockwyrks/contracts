# contracts

The data contracts every component reads and writes.

This repository is one component of
[The Test Cabinet](https://github.com/TheClockwyrks/TheTestCabinet). What it is
and how it fits with the others is stated by the documentation site, which
lives in the superrepo at `apps/docs/src/content/docs/`. Its Repositories page,
`apps/docs/src/content/docs/development/repositories.md`, draws the edges
between the repositories.

## What lives here

- `crates/contracts` (`test-cabinet-contracts`): the shapes more than one party
  reads or writes, the run record, the review, the event stream, the gg
  documents and the test suite model among them. It is the source of truth for
  the contract, with its fixtures under `crates/contracts/fixtures/`.
- `crates/suites` (`test-cabinet-suites`): the test-suite runtime, the engine
  catalog resolver, the validator runners and the browser driver.
- `crates/contract-codegen`: generates the TypeScript packages and the JSON
  Schemas from the contract's types (`npm run gen:contract`); the
  `contract-drift` gate fails when a committed copy is stale.
- `packages/run-record` (`@clockwyrks/run-record`) and `packages/asset-contract`
  (`@clockwyrks/asset-contract`): the generated TypeScript, published to the
  project's feed. The asset contract is seeded into a model's workspace, so
  `tests/asset-contract.test.ts` holds its prose to the seeded vocabulary.
- `schema/core/` and `schema/gg/`: the generated JSON Schemas, which the
  documentation site publishes under `/schema/`.
- `history/`: how this repository's history was extracted from the monorepo.

## Getting started

Open the superrepo in its devcontainer, which carries every toolchain the
repositories need, and work from this repository's root.
A build here inherits the superrepo's Cargo configuration, which redirects
every dependency on another repository to the sibling checkout, so nothing is
fetched from a mirror.

One command runs every gate:

```sh
make gate
```

Each gate is a file under `ci/gates/` whose stem is the gate's id.
`uv run --quiet --project ci gate list` names them all, and
`azure-pipelines.yml` runs each one as a step of its own on every push, inside
the CI images the superrepo builds.
