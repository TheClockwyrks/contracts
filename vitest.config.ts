// The workspace's tests: the root's own, under tests/. The packages carry no
// tests of their own.
//
// The include is narrowed to tests/ on purpose. Vitest's default include does
// not skip crates/, and the conformance fixture under
// crates/contracts/fixtures/test-suite/ holds a test suite's own validator
// tests, which run in a browser against the debug API of a built game and are
// not tests of this workspace.
//
// Given CI_GATE_ARTIFACTS (the typescript gate sets it), the run writes the JUnit
// report and the coverage summary into that directory, as ci/README.md names
// them; without it, it writes neither.
import { defineConfig } from "vitest/config";

const artifacts = process.env.CI_GATE_ARTIFACTS;

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    environment: "node",
    ...(artifacts
      ? {
          reporters: ["default", "junit"],
          outputFile: { junit: `${artifacts}/junit.xml` },
          coverage: {
            enabled: true,
            provider: "v8",
            reporter: ["json-summary"],
            reportsDirectory: artifacts,
            include: ["packages/*/src/**"],
          },
        }
      : {}),
  },
});
