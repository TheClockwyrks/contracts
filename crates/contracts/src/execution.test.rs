//! Tests for the execution contract's constants.

use super::*;

#[test]
fn the_implementation_commit_is_never_taken_for_the_seed() {
    assert_ne!(
        IMPLEMENTATION_COMMIT_MESSAGE.trim(),
        SEED_COMMIT_MESSAGE.trim()
    );
    assert!(!IMPLEMENTATION_COMMIT_MESSAGE.trim().is_empty());
    assert!(!IMPLEMENTATION_COMMIT_MESSAGE.contains('\n'));
}
