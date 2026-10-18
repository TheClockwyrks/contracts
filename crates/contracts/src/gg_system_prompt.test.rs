//! The backend API package's committed copy of gg's system-prompt templates is the text
//! this crate holds.

use super::{SYSTEM_CODE_TEMPLATE, SYSTEM_TOOLS_TEMPLATE};

/// The generated-and-committed copy of both templates the console seeds its editors from.
///
/// `scripts/gen-contract.mjs` writes it out of the two `.hbs` files gg embeds, and the
/// `contract-drift` gate regenerates and diffs it. It is included here so the staleness is a
/// *suite* failure as well, for the reason it went unnoticed once: the drift check is reached for
/// when somebody thinks a contract **type** moved, and a template is not a type.
const COMMITTED_TEMPLATES: &str =
    include_str!("../../../packages/backend-api/src/gg-system-prompt.ts");

/// The string literal `name` is assigned in [`COMMITTED_TEMPLATES`], unescaped.
///
/// Written out rather than taken from a parser because the escaping is the whole of it: the
/// generator writes `JSON.stringify(template)` and prettier then re-quotes the literal to whichever
/// quote character needs fewer escapes, so a byte comparison against either spelling would be a test
/// about prettier. Unescaping answers the question actually being asked — *is the text in this file
/// the text gg embeds* — in either spelling.
fn committed_template(name: &str) -> String {
    let assignment = format!("export const {name} =");
    let tail = COMMITTED_TEMPLATES
        .split_once(&assignment)
        .unwrap_or_else(|| {
            panic!("{name} is exported by packages/backend-api/src/gg-system-prompt.ts")
        })
        .1;
    let opened = tail
        .find(['\'', '"'])
        .expect("the constant is assigned a string literal");
    let quote = tail.as_bytes()[opened] as char;
    let mut text = String::new();
    let mut characters = tail[opened + 1..].chars();
    while let Some(character) = characters.next() {
        match character {
            _ if character == quote => return text,
            '\\' => match characters
                .next()
                .expect("an escape is not the last character")
            {
                'n' => text.push('\n'),
                't' => text.push('\t'),
                'r' => text.push('\r'),
                escaped => text.push(escaped),
            },
            _ => text.push(character),
        }
    }
    panic!("the string literal assigned to {name} is never closed");
}

/// **What the console shows an operator is what gg would render.**
///
/// The console seeds its per-agent System Prompt editor from these two constants and stores an
/// override whenever the text an operator saves differs from them — and
/// gg's `render_system` runs an override *instead of* gg's own template. So a
/// committed copy that has fallen behind is not a stale comment: it is a path from a template edit
/// to a real model reading a paragraph describing an arrangement this tree deleted. That is not
/// hypothetical either — the eleventh and last arm to convert on this branch edited
/// `system-code.hbs` and did not regenerate, and every gate but the drift script stayed green.
#[test]
fn the_committed_copy_of_each_template_is_the_one_gg_embeds() {
    for (name, embedded) in [
        ("DEFAULT_GG_SYSTEM_PROMPT_TEMPLATE", SYSTEM_TOOLS_TEMPLATE),
        (
            "DEFAULT_GG_SYSTEM_PROMPT_TEMPLATE_CODE",
            SYSTEM_CODE_TEMPLATE,
        ),
    ] {
        let committed = committed_template(name);
        // The first line that differs rather than `assert_eq!`, because these are ~500-line
        // documents and the whole of both printed twice buries the one line that moved.
        let differing = committed
            .lines()
            .zip(embedded.lines())
            .enumerate()
            .find(|(_, (committed, embedded))| committed != embedded)
            .map(|(index, (committed, embedded))| {
                format!(
                    "first at line {}:\n  committed: {committed}\n  gg embeds: {embedded}",
                    index + 1
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "the committed copy is {} lines and gg's is {}",
                    committed.lines().count(),
                    embedded.lines().count()
                )
            });
        assert!(
            committed == embedded,
            "{name} in packages/backend-api/src/gg-system-prompt.ts is not the template gg embeds \
             — {differing}\n\nRun `npm run gen:contract` and commit the result."
        );
    }
}
