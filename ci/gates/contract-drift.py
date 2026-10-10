"""Fail when a committed, generated data-contract artifact is stale.

The TypeScript bindings in `packages/run-record/src/` and
`packages/asset-contract/src/` and the JSON Schemas in `schema/` are generated
from the Rust types in `crates/contracts` by `crates/contract-codegen`, and
committed because their readers cannot run the generator.
scripts/ci/contract-drift.sh regenerates them and fails on any difference, or on
a generated file that was never committed. It needs cargo and Node, and the
workspace installed for Prettier and tsc, so it runs on the rust track, whose
setup steps provide both and run `npm ci`.

The gate is this repository's own, not the kit's: the pipeline renders a step
for it from the kit's table of extra gates, and an update of the kit never
touches this file.
"""

from the_test_cabinet_ci import GIT_LOCATION_VARIABLES, enter_repo_root, fail, require_tool, run

enter_repo_root()
require_tool("cargo", "The devcontainer and the Rust CI image install it.")
require_tool("node", "The devcontainer and the rust-browser CI image install it.")

if run(["scripts/ci/contract-drift.sh"], unset=GIT_LOCATION_VARIABLES).returncode != 0:
    fail(
        "",
        "A generated contract artifact is stale. Regenerate it and commit the result:",
        "    npm run gen:contract",
    )
