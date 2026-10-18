//! The harness contract's names: what each harness is called, the environment
//! variables that carry its API key, and the environment variables that pin a run's
//! container image.
//!
//! Each is read on more than one side. A harness's display name is shown by the
//! CLI, the console and the gallery. The key variables are what the driver injects
//! a key from and what redaction scrubs the values of before a run's source or
//! logs leave for a public destination. The image-override variables are read by
//! the driver's run-image resolution and forwarded into every driver Job by the
//! dispatcher. One definition here keeps those sides agreeing.

use crate::run_record::HarnessSlug;

// --- display names ------------------------------------------------------------

impl HarnessSlug {
    /// The human-readable name of this harness, for display: the name the CLI's
    /// harness listing, the console and the gallery show it under.
    pub fn display_name(self) -> &'static str {
        match self {
            HarnessSlug::Claude => "Anthropic Claude Code",
            HarnessSlug::Codex => "OpenAI Codex",
            HarnessSlug::Cline => "Cline",
            HarnessSlug::Antigravity => "Google Antigravity",
            HarnessSlug::Goose => "Goose",
            HarnessSlug::Kilo => "Kilo Code",
            HarnessSlug::Opencode => "OpenCode",
            HarnessSlug::Pi => "Pi",
            HarnessSlug::Gg => "gg",
        }
    }
}

// --- API keys ------------------------------------------------------------------

/// The environment variable a harness's provider API key is read from on the host,
/// or `None` for a harness that authenticates only through a subscription
/// (Antigravity).
///
/// Every harness that reaches its model through OpenRouter, gg included, shares
/// `OPENROUTER_API_KEY`; [`api_key_override_var`] gives each one a variable of its
/// own that takes precedence.
pub fn api_key_env(slug: HarnessSlug) -> Option<&'static str> {
    match slug {
        HarnessSlug::Claude => Some("ANTHROPIC_API_KEY"),
        HarnessSlug::Codex => Some("OPENAI_API_KEY"),
        HarnessSlug::Antigravity => None,
        HarnessSlug::Cline
        | HarnessSlug::Goose
        | HarnessSlug::Kilo
        | HarnessSlug::Opencode
        | HarnessSlug::Pi
        | HarnessSlug::Gg => Some("OPENROUTER_API_KEY"),
    }
}

/// The per-harness API-key override variable, `TCAB_API_KEY_<SLUG>` (for example
/// `TCAB_API_KEY_KILO`). When set, it supplies that one harness's key, taking
/// precedence over the shared provider variable ([`api_key_env`]) so harnesses that
/// share a provider can be given independent keys.
pub fn api_key_override_var(slug: HarnessSlug) -> String {
    format!("TCAB_API_KEY_{}", slug.as_str().to_ascii_uppercase())
}

/// The names of every environment variable whose value redaction scrubs: each
/// runnable harness's provider key variable and its per-harness override, sorted and
/// without duplicates.
///
/// These are the variables a run's key can come from, so a value set in any of them
/// on the host that released a run is one that may have reached the run's output.
pub fn redacted_env_vars() -> Vec<String> {
    let mut vars = Vec::new();
    for slug in HarnessSlug::RUNNABLE {
        if let Some(env) = api_key_env(slug) {
            vars.push(env.to_string());
        }
        vars.push(api_key_override_var(slug));
    }
    vars.sort();
    vars.dedup();
    vars
}

// --- run-image overrides -----------------------------------------------------------

/// The suffix that names a gg variant's override environment variable, the counterpart of
/// the `-gg` suffix that names the variant's image. A gg variant is pinned on its own, exactly as every other
/// image is: it is a different image with a different build, and an override that covered
/// both it and its parent could only ever be right for one.
pub const GG_IMAGE_OVERRIDE_ENV_SUFFIX: &str = "_GG";

/// The environment variable that pins a verbatim override for the base-wasm
/// (end-to-end) image, the per-image counterpart of `TCAB_CONTAINER_REGISTRY`/`_TAG`.
pub const BASE_WASM_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_BASE_WASM";
/// The environment variable that pins a verbatim override for the full-stack (2D)
/// image.
pub const FULL_STACK_2D_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_FULL_STACK_2D";
/// The environment variable that pins a verbatim override for the 3D full-stack image.
/// Separate from the 2D one for the reason every per-image override is separate: the two
/// full-stack images are different builds, so an override that covered both could only
/// ever be right for one.
pub const FULL_STACK_3D_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_FULL_STACK_3D";
/// The environment variable that pins a verbatim override for the game-jam image.
pub const GAME_JAM_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_GAME_JAM";
/// The environment variable that pins a verbatim override for the sprite image.
pub const SPRITE_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SPRITE";
/// The environment variable that pins a verbatim override for the sprite-sheet
/// image.
pub const SPRITE_SHEET_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SPRITE_SHEET";
/// The environment variable that pins a verbatim override for the static-voxel image.
pub const VOXEL_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_VOXEL";
/// The environment variable that pins a verbatim override for the animated-voxel image.
pub const VOXEL_ANIMATION_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_VOXEL_ANIMATION";
/// The environment variable that pins a verbatim override for the marching-cubes
/// static (`mc-model`) image.
pub const MC_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_MC";
/// The environment variable that pins a verbatim override for the marching-cubes
/// animated (`mc-animation`) image.
pub const MC_ANIMATION_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_MC_ANIMATION";
/// The environment variable that pins a verbatim override for the surface-nets
/// static (`sn-model`) image.
pub const SN_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SN";
/// The environment variable that pins a verbatim override for the surface-nets
/// animated (`sn-animation`) image.
pub const SN_ANIMATION_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SN_ANIMATION";
/// The environment variable that pins a verbatim override for the dual-contouring
/// static (`dc-model`) image.
pub const DC_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_DC";
/// The environment variable that pins a verbatim override for the dual-contouring
/// animated (`dc-animation`) image.
pub const DC_ANIMATION_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_DC_ANIMATION";
/// The environment variable that pins a verbatim override for the `ui` image.
pub const UI_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_UI";
/// The environment variable that pins a verbatim override for the `material` image.
pub const MATERIAL_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_MATERIAL";
/// The environment variable that pins a verbatim override for the `mc-skinned` image.
pub const MC_SKINNED_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_MC_SKINNED";
/// The environment variable that pins a verbatim override for the `sn-skinned` image.
pub const SN_SKINNED_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SN_SKINNED";
/// The environment variable that pins a verbatim override for the `dc-skinned` image.
pub const DC_SKINNED_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_DC_SKINNED";
/// The environment variable that pins a verbatim override for the `particle-2d` image.
pub const PARTICLE_2D_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_PARTICLE_2D";
/// The environment variable that pins a verbatim override for the `particle-3d` image.
pub const PARTICLE_3D_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_PARTICLE_3D";
/// The environment variable that pins a verbatim override for the `sfx-synth` image.
pub const SFX_SYNTH_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SFX_SYNTH";
/// The environment variable that pins a verbatim override for the `sfx-sample` image.
pub const SFX_SAMPLE_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_SFX_SAMPLE";
/// The environment variable that pins a verbatim override for the `music` image.
pub const MUSIC_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_MUSIC";
/// The environment variable that pins a verbatim override for the `blender` image.
pub const BLENDER_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_BLENDER";
/// The environment variable that pins a verbatim override for the adversarial
/// image.
pub const ADVERSARIAL_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_ADVERSARIAL";
/// The environment variable that pins a verbatim override for the performance
/// image.
pub const PERFORMANCE_IMAGE_OVERRIDE_ENV: &str = "TCAB_CONTAINER_IMAGE_PERFORMANCE";

/// Every per-image override environment variable a run's own image can be pinned
/// with, one per run image, in image build order — **without** the gg variants, which
/// [`RUN_IMAGE_OVERRIDE_ENVS`] derives from these.
///
/// It lives beside the override constants, so a new run image is named in ONE place;
/// the driver's `run_image_override_envs_is_exhaustive` test fails the build if any
/// kind's resolved override variable is missing from this list (or vice versa), so
/// the dispatcher's forwarded set can never again silently drift behind the images
/// that exist.
pub const RUN_IMAGE_OVERRIDE_ENVS_PLAIN: &[&str] = &[
    BASE_WASM_IMAGE_OVERRIDE_ENV,
    FULL_STACK_2D_IMAGE_OVERRIDE_ENV,
    FULL_STACK_3D_IMAGE_OVERRIDE_ENV,
    GAME_JAM_IMAGE_OVERRIDE_ENV,
    SPRITE_IMAGE_OVERRIDE_ENV,
    SPRITE_SHEET_IMAGE_OVERRIDE_ENV,
    VOXEL_IMAGE_OVERRIDE_ENV,
    VOXEL_ANIMATION_IMAGE_OVERRIDE_ENV,
    MC_IMAGE_OVERRIDE_ENV,
    MC_ANIMATION_IMAGE_OVERRIDE_ENV,
    SN_IMAGE_OVERRIDE_ENV,
    SN_ANIMATION_IMAGE_OVERRIDE_ENV,
    DC_IMAGE_OVERRIDE_ENV,
    DC_ANIMATION_IMAGE_OVERRIDE_ENV,
    UI_IMAGE_OVERRIDE_ENV,
    MATERIAL_IMAGE_OVERRIDE_ENV,
    MC_SKINNED_IMAGE_OVERRIDE_ENV,
    SN_SKINNED_IMAGE_OVERRIDE_ENV,
    DC_SKINNED_IMAGE_OVERRIDE_ENV,
    PARTICLE_2D_IMAGE_OVERRIDE_ENV,
    PARTICLE_3D_IMAGE_OVERRIDE_ENV,
    SFX_SYNTH_IMAGE_OVERRIDE_ENV,
    SFX_SAMPLE_IMAGE_OVERRIDE_ENV,
    MUSIC_IMAGE_OVERRIDE_ENV,
    BLENDER_IMAGE_OVERRIDE_ENV,
    ADVERSARIAL_IMAGE_OVERRIDE_ENV,
    PERFORMANCE_IMAGE_OVERRIDE_ENV,
];

/// Every per-image override environment variable the driver's run-image resolution
/// consults: each
/// run image's own, and each one's `_GG` counterpart pinning that image's **gg variant**
/// (the same image plus the language toolchains a `gg` run's programs are compiled with).
/// This is the **canonical set** a deployment must
/// forward from the dispatcher into each driver `Job` so a full-ref
/// `TCAB_CONTAINER_IMAGE_*` override actually reaches the run-image resolution that
/// runs in the driver (see the dispatcher's `PASSTHROUGH_K8S_VARS`) — the
/// `TCAB_CONTAINER_REGISTRY`/`_TAG` composition is forwarded separately.
///
/// The variants are derived rather than listed, for the same reason their image names
/// are: there is exactly one per run image and no set of images to keep in step.
pub static RUN_IMAGE_OVERRIDE_ENVS: std::sync::LazyLock<Vec<String>> =
    std::sync::LazyLock::new(|| {
        RUN_IMAGE_OVERRIDE_ENVS_PLAIN
            .iter()
            .flat_map(|env| {
                [
                    (*env).to_string(),
                    format!("{env}{GG_IMAGE_OVERRIDE_ENV_SUFFIX}"),
                ]
            })
            .collect()
    });

#[cfg(test)]
#[path = "harness.test.rs"]
mod tests;
