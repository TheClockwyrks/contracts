//! Tests for the harness contract's names.

use super::*;

#[test]
fn every_harness_has_a_display_name_of_its_own() {
    let mut names: Vec<&str> = HarnessSlug::RUNNABLE
        .iter()
        .map(|slug| slug.display_name())
        .collect();
    assert!(names.iter().all(|name| !name.trim().is_empty()));
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), HarnessSlug::RUNNABLE.len());
}

#[test]
fn the_override_variable_is_named_after_the_slug() {
    assert_eq!(api_key_override_var(HarnessSlug::Kilo), "TCAB_API_KEY_KILO");
    assert_eq!(
        api_key_override_var(HarnessSlug::Opencode),
        "TCAB_API_KEY_OPENCODE"
    );
    assert_eq!(api_key_override_var(HarnessSlug::Gg), "TCAB_API_KEY_GG");
}

/// An OpenRouter harness is exactly one that reads `OPENROUTER_API_KEY`, which is
/// what decides how its model ids are canonicalized.
#[test]
fn the_openrouter_harnesses_are_the_ones_reading_the_openrouter_key() {
    for slug in HarnessSlug::RUNNABLE {
        assert_eq!(
            api_key_env(slug) == Some("OPENROUTER_API_KEY"),
            slug.routes_through_openrouter(),
            "{slug:?}"
        );
    }
}

#[test]
fn redaction_scrubs_every_key_variable_and_every_override() {
    let vars = redacted_env_vars();
    for slug in HarnessSlug::RUNNABLE {
        if let Some(env) = api_key_env(slug) {
            assert!(vars.iter().any(|var| var == env), "{env}");
        }
        let over = api_key_override_var(slug);
        assert!(vars.contains(&over), "{over}");
    }
    let mut sorted = vars.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(vars, sorted, "sorted and without duplicates");
}

#[test]
fn every_run_image_override_has_a_gg_variant_beside_it() {
    assert_eq!(
        RUN_IMAGE_OVERRIDE_ENVS.len(),
        RUN_IMAGE_OVERRIDE_ENVS_PLAIN.len() * 2
    );
    for env in RUN_IMAGE_OVERRIDE_ENVS_PLAIN {
        assert!(RUN_IMAGE_OVERRIDE_ENVS.contains(&env.to_string()), "{env}");
        let gg = format!("{env}{GG_IMAGE_OVERRIDE_ENV_SUFFIX}");
        assert!(RUN_IMAGE_OVERRIDE_ENVS.contains(&gg), "{gg}");
    }
}
