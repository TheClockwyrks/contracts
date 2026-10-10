// Generate the data contract, the TypeScript bindings and the JSON Schemas, from
// the Rust types that are its single source of truth.
//
// The work happens in three steps. `contract-codegen` (`crates/contract-codegen`)
// emits the TypeScript (`packages/run-record/src/`, `packages/asset-contract/src/`)
// and the JSON Schemas (`schema/core/`, `schema/gg/`) from `crates/contracts`,
// whose types derive `ts_rs::TS` + `schemars::JsonSchema` behind its `contract`
// feature. The generator is correct but not pretty (ts_rs emits ragged
// whitespace), so the second step runs Prettier to make the committed output
// deterministic and conventional. CI regenerates and fails on any diff
// (`scripts/ci/contract-drift.sh`), so the committed artifacts always match the
// Rust source and hand edits never survive. The last step compiles the
// regenerated TypeScript, because `dist/`, not `src/`, is what every consumer of
// the packages imports.
//
// Usage: `npm run gen:contract` (from the repository root).

import { execFileSync } from "node:child_process";

function run(cmd, args) {
  execFileSync(cmd, args, { stdio: "inherit" });
}

// 1. Emit raw TS + JSON from the Rust source of truth.
run("cargo", ["run", "--quiet", "-p", "contract-codegen"]);

// 2. Normalize formatting so the committed artifacts are deterministic. Prettier
//    is pinned (see the root `package.json`) so the same input always formats the
//    same way, which is what makes the drift check trustworthy. Every file listed
//    here is one the drift check diffs, and that is the rule: an artifact it guards
//    has to come out of the same formatter, or a byte of whitespace turns CI red.
run("npx", [
  "prettier",
  "--write",
  "--log-level",
  "warn",
  "packages/run-record/src/**/*.ts",
  "packages/asset-contract/src/**/*.ts",
  "schema/**/*.json",
]);

// 3. Compile the TypeScript that was just regenerated. Each package's `exports`
//    map points at `dist/`, so emitting only the source leaves every consumer
//    building against the previous contract until something happens to compile
//    it. Regenerating and compiling are one act; do not split them. run-record
//    references asset-contract, so this builds both.
run("npx", ["tsc", "-b", "packages/run-record"]);
