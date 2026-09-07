//! Validate the actual shared palette, rather than a second list of RGB values.
use std::collections::BTreeMap;

const PRESET: &str = include_str!("../../../assets/theme/preset.css");
const STATUS: &str = include_str!("../../../assets/theme/tokens.css");

fn declarations(block: &str) -> BTreeMap<&str, &str> {
    block
        .lines()
        .filter_map(|line| {
            let (key, value) = line.trim().strip_prefix("--")?.split_once(':')?;
            Some((key, value.trim().trim_end_matches(';')))
        })
        .collect()
}

fn luminance(value: &str) -> f64 {
    let hex = value
        .strip_prefix('#')
        .expect("contrast parser requires hex; extend for new export formats");
    assert_eq!(hex.len(), 6);
    let channels: Vec<f64> = (0..3)
        .map(|i| {
            let c = f64::from(u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex channel"))
                / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        })
        .collect();
    channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
}

#[test]
fn both_appearances_keep_text_and_status_readable() {
    let light = declarations(
        PRESET.split_once(":root {").expect("light").1.split('}').next().expect("block"),
    );
    let dark = declarations(
        PRESET.split_once(".dark {").expect("dark").1.split('}').next().expect("block"),
    );
    let status_light = declarations(
        STATUS.split_once(":root {").expect("status light").1.split('}').next().expect("block"),
    );
    let status_dark = declarations(
        STATUS.split_once(".dark {").expect("status dark").1.split('}').next().expect("block"),
    );
    for (mode, mut colors, status) in [("light", light, status_light), ("dark", dark, status_dark)]
    {
        colors.extend(status.into_iter().filter(|(_, value)| value.starts_with('#')));
        for (fg, bg) in [
            ("foreground", "background"),
            ("card-foreground", "card"),
            ("popover-foreground", "popover"),
            ("muted-foreground", "background"),
            ("muted-foreground", "card"),
            ("muted-foreground", "popover"),
            ("primary-foreground", "primary"),
            ("primary", "background"),
            ("primary", "card"),
            ("accent-foreground", "accent"),
            ("destructive-foreground", "destructive"),
            ("sidebar-foreground", "sidebar"),
            ("sidebar-primary-foreground", "sidebar-primary"),
            ("signal", "signal-soft"),
            ("warning", "warning-soft"),
            ("danger", "danger-soft"),
            ("signal", "card"),
            ("warning", "card"),
            ("danger", "card"),
        ] {
            let a = luminance(colors[fg]);
            let b = luminance(colors[bg]);
            let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            assert!(ratio >= 4.5, "{mode} {fg} on {bg}: contrast {ratio:.2}");
        }
    }
}
