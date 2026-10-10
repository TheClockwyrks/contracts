# history

The record of how this repository's history was extracted from the monorepo,
[The Test Cabinet](https://github.com/TheClockwyrks/TheTestCabinet), where the
crates and packages here lived until the extraction. Nothing builds or tests
against these files; they are kept so the provenance outlives the scratch clone
the extraction ran in.

## The extraction

The history below the scaffold commit is the monorepo's `nightly` at
`3871188d36cf940a5ac46fdf596f43aafa3ea18a`, filtered to the paths in
[`paths.txt`](paths.txt) by git filter-repo 2.47.0:

```sh
git clone -q --no-local --single-branch --branch nightly --no-tags <the-test-cabinet> x
cd x
uvx --from git-filter-repo==2.47.0 git-filter-repo --force \
  --paths-from-file ../paths.txt --prune-empty always --preserve-commit-hashes
```

Of the 6,667 commits reachable from that commit, 553 touched a path in the list
and were kept (67 of them merges); the rest were pruned. No tag was carried. The
tip tree of every moved directory hashes the same as the monorepo's at that
commit.

The paths are kept where they were in the monorepo, so the filter needed no
rename and every relative path inside the crates (the fixtures, an
`include_str!`) works as it did.

## The files

- [`paths.txt`](paths.txt) is the list handed to `--paths-from-file`
  (sha256 `4007cc7af1d9cbfe91a5fc3e24b7cddf859d0e6d2b97789f1c541e047e3968fe`).
  It is generated, not hand-written: the five moved directories
  (`crates/contracts/`, `crates/suites/`, `crates/contract-codegen/`,
  `packages/run-record/`, `packages/asset-contract/`), and every path a tracked
  file under them had before a rename or copy, collected by
  `git log --follow --name-status` over each of the 188 files, so a file's
  history reaches back past the commits that moved it into place.
- [`commit-map`](commit-map) is filter-repo's map from each monorepo commit (left)
  to the commit it became here (right), all zeros for a pruned one.
  `--preserve-commit-hashes` left the commit ids quoted in the messages as they
  were, so an id in a message below the scaffold commit names a monorepo commit,
  and this map translates it.

## The live-path review

`paths.txt` intersected with the monorepo's tree at the extraction commit names,
outside the five moved directories, exactly five live files:

```text
crates/core/src/browser.rs
crates/core/src/gg_query.rs
crates/core/src/run_record.rs
crates/core/src/run_record.test.rs
crates/core/src/validation.rs
```

Each is a copy source: a contracts or suites file began as a copy of it when the
crates were split out of `crates/core`, and the monorepo kept its own copy. They
were kept in the list so `git log --follow` and blame of the files copied from
them reach their history before the split. Their commits after the split, which
touch the monorepo's copies only, are in this history too, and the scaffold
commit deletes the five files.
