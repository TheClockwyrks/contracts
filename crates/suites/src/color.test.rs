use super::*;

#[test]
fn canvas_accepts_transparent_in_any_case() {
    assert_eq!(
        CanvasBackground::parse("transparent"),
        Ok(CanvasBackground::Transparent)
    );
    assert_eq!(
        CanvasBackground::parse("TRANSPARENT"),
        Ok(CanvasBackground::Transparent)
    );
}

#[test]
fn canvas_reads_an_opaque_and_a_translucent_color() {
    assert_eq!(
        CanvasBackground::parse("#1A2b3c"),
        Ok(CanvasBackground::Color([0x1a, 0x2b, 0x3c, 0xff]))
    );
    assert_eq!(
        CanvasBackground::parse("#1a2b3c80"),
        Ok(CanvasBackground::Color([0x1a, 0x2b, 0x3c, 0x80]))
    );
}

#[test]
fn canvas_refuses_what_the_drawing_tool_refuses() {
    for value in [
        "1a2b3c",
        "#1a2b3",
        "#1a2b3c8",
        "#1a2b3c8000",
        "#gg0000",
        "",
        "#",
        "white",
    ] {
        let err = CanvasBackground::parse(value).expect_err(value);
        assert_eq!(
            err.to_string(),
            format!("invalid color `{value}` (expected `#rrggbb`, `#rrggbbaa`, or `transparent`)")
        );
    }
}

#[test]
fn preview_reads_an_opaque_color() {
    assert_eq!(
        PreviewBackground::parse("Transparent"),
        Ok(PreviewBackground::Transparent)
    );
    assert_eq!(
        PreviewBackground::parse("#FFffFF"),
        Ok(PreviewBackground::Color([0xff, 0xff, 0xff]))
    );
}

#[test]
fn preview_refuses_an_alpha_component() {
    let err = PreviewBackground::parse("#ffffff80").expect_err("alpha is refused");
    assert_eq!(
        err.to_string(),
        "invalid color `#ffffff80` (expected `#rrggbb` or `transparent`)"
    );
}

#[test]
fn preview_refuses_what_the_model_core_refuses() {
    for value in ["ffffff", "#fff", "#fffffff", "#zzzzzz", "", "#"] {
        assert_eq!(
            PreviewBackground::parse(value),
            Err(ColorError::Preview(value.to_string()))
        );
    }
}
