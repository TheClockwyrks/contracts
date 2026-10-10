"""Run the Rust test suite with cargo nextest.

nextest is the test runner here: `.config/nextest.toml` holds the retries, the
no-fail-fast rule and the per-test hard timeout, none of which the built-in
runner has.

A crate with no tests yet passes rather than failing the gate, so a fresh
repository is green until its first test arrives.

Asked for artifacts, the run uses the `ci` profile, which inherits the default
one and also writes a JUnit report. Coverage is deliberately left alone: it
costs a second, instrumented compilation of the workspace.
"""

import shutil

from the_test_cabinet_ci import artifacts_dir, enter_repo_root, fail, run

# From the workspace root, so cargo resolves the workspace regardless of the
# caller's working directory.
root = enter_repo_root()

command = ["cargo", "nextest", "run", "--workspace", "--no-tests=pass"]
artifacts = artifacts_dir()
# nextest keeps its reports under the checkout (its `store.dir` is relative to
# the workspace root), and not under cargo's target directory.
report = root / "target" / "nextest" / "ci" / "junit.xml"
if artifacts is not None:
    command += ["--profile", "ci"]
    # A run that dies before its first test leaves the report of the run before
    # it in place, and that one must not be taken for this run's.
    report.unlink(missing_ok=True)

status = run(command).returncode

if artifacts is not None and report.is_file():
    shutil.copyfile(report, artifacts / "junit.xml")

if status != 0:
    fail("", "The Rust tests failed. Fix them, then run this gate again.")
