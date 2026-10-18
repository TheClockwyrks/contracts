//! gg's built-in system-prompt templates: the default an agent's
//! [`system_prompt_template`](crate::gg::GgAgentConfig::system_prompt_template) override
//! starts from.
//!
//! There are two, chosen by the agent's execution mode: the tool-calling arm names each
//! capability's free-standing tools, and the responses-as-code arm teaches the code
//! protocol. Both are language-agnostic, so there is one file per mode. gg embeds and
//! renders them (see the documentation site's `gg/prompts` page), and the backend API
//! package mirrors them verbatim (`packages/backend-api/src/gg-system-prompt.ts`,
//! written by `npm run gen:contract`) so the console can seed its per-agent System
//! Prompt editor with the exact default. They live here, not in gg, because the
//! mirror is generated in this repository and both readers name it.
//!
//! Each is self-contained Handlebars: partials are unavailable by design, so the
//! console can seed an override editor with one standalone file.

/// The **tool-calling** system prompt: the base framing plus one conditional section per
/// enabled capability, with each capability's tools named as the free-standing calls a
/// tool-calling run makes.
pub const SYSTEM_TOOLS_TEMPLATE: &str = include_str!("../templates/gg/system-tools.hbs");

/// The **responses-as-code** system prompt: the code-protocol framing plus the same
/// capability sections as the tool-calling arm, describing each capability rather than
/// naming the calls behind it. One file serves every program language.
pub const SYSTEM_CODE_TEMPLATE: &str = include_str!("../templates/gg/system-code.hbs");

#[cfg(test)]
#[path = "gg_system_prompt.test.rs"]
mod tests;
