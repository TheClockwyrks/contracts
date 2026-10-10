// The seeded contract package must not name this project or say anything about
// how a run is judged.
//
// `@clockwyrks/asset-contract` describes the rig a produced model is made of, and
// the voxel and particle runtimes depend on it. Those runtimes are vendored into
// a model's own workspace when a run is seeded, so every byte of this package
// travels with them and is readable by the model building the case: a model must
// not learn that it is being evaluated, that its output is scored, or that the
// project exists. The package is generated from Rust doc comments by
// `crates/contract-codegen`, which copies them through verbatim, so a `///` line
// added to one of these types on the Rust side lands in a run workspace unless
// something looks. This is what looks.
//
// The rest of the contract (`@clockwyrks/run-record`: records, reviews) is
// deliberately not checked here: it is never seeded, and it is supposed to talk
// about evaluation.
//
// To fix a failure, reword the Rust doc comment so it describes the shape rather
// than the machinery around it, and move any maintainer-facing note to a plain
// `//` comment beneath it, which is not copied. `crates/contracts/src/test_case.rs`'s
// `ModelSpec` is the worked example. The engines and runtimes the package is
// seeded with are checked where they live, by the superrepo's `seeded-contract`
// gate.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const PACKAGE = join(ROOT, "packages", "asset-contract");
const SOURCE = join(PACKAGE, "src");

// No word here has an innocent reading in a file that describes a rig, so the
// list can be broad, and being broad is the point, since its prose comes from
// Rust doc comments written by someone thinking about the harness. It is the
// STRICT tier of the superrepo's `scripts/ci/seeded-contract-check.sh`.
const STRICT =
  /test.cabinet|testcabinet|\btcab\b|\bbenchmark(s|ed|ing)?\b|\bevaluat(e|ed|es|ing|ion)\b|\bscor(e|ed|es|ing)\b|\bgrad(e|ed|es|ing)\b|\breview(s|ed|er|ers|ing)?\b|\brun.record\b|\bleaderboard\b|\bladder\b|\bharness\b|https?:\/\//i;

function files(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

describe("the seeded asset contract", () => {
  it("holds the generated TypeScript", () => {
    // A relocated or emptied package would otherwise pass the checks below
    // having checked nothing.
    expect(existsSync(SOURCE), `${SOURCE} does not exist`).toBe(true);
    expect(files(SOURCE).filter((path) => path.endsWith(".ts"))).not.toEqual(
      [],
    );
  });

  it("names nothing a model must not learn", () => {
    const hits = files(SOURCE).flatMap((path) =>
      readFileSync(path, "utf8")
        .split("\n")
        .flatMap((line, index) =>
          STRICT.test(line)
            ? [`${relative(ROOT, path)}:${index + 1}: ${line.trim()}`]
            : [],
        ),
    );
    expect(hits).toEqual([]);
  });

  it("depends on no package at run time", () => {
    // Seeding copies a package's whole dependency closure into the run
    // workspace, so one `dependencies` entry is all it would take for the
    // evaluation contract to land there.
    const manifest = JSON.parse(
      readFileSync(join(PACKAGE, "package.json"), "utf8"),
    ) as {
      dependencies?: Record<string, string>;
      peerDependencies?: Record<string, string>;
    };
    expect(manifest.dependencies ?? {}).toEqual({});
    expect(manifest.peerDependencies ?? {}).toEqual({});
  });

  it("imports nothing", () => {
    const imports = files(SOURCE).flatMap((path) =>
      readFileSync(path, "utf8")
        .split("\n")
        .flatMap((line, index) =>
          /^\s*(import|export)\b.*\bfrom\s+["']|^\s*import\s+["']|\bimport\(/.test(
            line,
          )
            ? [`${relative(ROOT, path)}:${index + 1}: ${line.trim()}`]
            : [],
        ),
    );
    expect(imports).toEqual([]);
  });
});
