use super::*;
use crate::gg::{GgAgentConfig, GgCapabilitySet, GgSlotCost};
use crate::metrics::{Cost, TokenCounts};

/// An agent profile named `slug`, bound to `model`.
fn gg_agent(slug: &str, model: &str) -> GgAgentConfig {
    GgAgentConfig {
        slug: slug.to_string(),
        name: format!("{slug} agent"),
        model_id: model.to_string(),
        ..GgAgentConfig::root()
    }
}

/// A slot cost of `tokens` uncached input tokens costing `cost`, on `profile`'s `model`.
fn slot_cost(profile: &str, model: &str, tokens: u64, cost: f64) -> GgSlotCost {
    GgSlotCost {
        profile_id: profile.to_string(),
        model_id: model.to_string(),
        tokens: TokenCounts {
            uncached_input: Some(tokens),
            ..Default::default()
        },
        cost: Some(Cost {
            comparable: Some(cost),
            actual: Some(cost),
        }),
        work_cost: None,
    }
}

#[test]
fn gg_agent_lines_list_every_declared_agent_in_order_with_its_spend() {
    let set = GgCapabilitySet {
        agents: vec![
            gg_agent("root", "opus"),
            gg_agent("reviewer", "gpt"),
            gg_agent("tester", "haiku"),
        ],
        ..GgCapabilitySet::default()
    };
    // The costs arrive in first-seen order, which is not the configuration's.
    let lines = gg_agent_lines(
        &set,
        &[
            slot_cost("reviewer", "gpt", 200, 0.25),
            slot_cost("root", "opus", 100, 1.5),
            slot_cost("reviewer", "gpt", 50, 0.25),
        ],
    );

    let summary: Vec<_> = lines
        .iter()
        .map(|line| {
            (
                line.profile.as_str(),
                line.primary,
                line.ran,
                line.tokens.uncached_input,
                line.cost.comparable,
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            ("root", true, true, Some(100), Some(1.5)),
            ("reviewer", false, true, Some(250), Some(0.5)),
            // Declared but never started: listed, marked, and costing nothing.
            ("tester", false, false, None, None),
        ]
    );
    assert_eq!(lines[0].name, "root agent");
    // A profile that never ran is shown on the model it was bound to.
    assert_eq!(lines[2].model_ids, vec!["haiku".to_string()]);
}

#[test]
fn gg_agent_lines_keep_every_model_a_re_pointed_profile_ran_on() {
    let set = GgCapabilitySet {
        agents: vec![gg_agent("root", "opus")],
        ..GgCapabilitySet::default()
    };
    let lines = gg_agent_lines(
        &set,
        &[
            slot_cost("root", "opus", 100, 1.0),
            slot_cost("root", "sonnet", 40, 0.25),
        ],
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].model_ids, vec!["opus", "sonnet"]);
    assert_eq!(lines[0].tokens.uncached_input, Some(140));
    assert_eq!(lines[0].cost.actual, Some(1.25));
}

#[test]
fn gg_agent_lines_list_spend_on_a_profile_the_configuration_does_not_declare() {
    let set = GgCapabilitySet {
        agents: vec![gg_agent("root", "opus")],
        ..GgCapabilitySet::default()
    };
    let lines = gg_agent_lines(
        &set,
        &[
            slot_cost("root", "opus", 100, 1.0),
            slot_cost("stray", "haiku", 10, 0.01),
        ],
    );
    let stray = lines.last().expect("a line for the undeclared profile");
    assert_eq!(stray.profile, "stray");
    assert_eq!(stray.name, "stray");
    assert!(stray.ran && !stray.primary);
}
