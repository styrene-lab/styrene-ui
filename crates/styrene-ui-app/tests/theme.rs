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
    let linear: Vec<f64> = if let Some(hex) = value.strip_prefix('#') {
        assert_eq!(hex.len(), 6);
        (0..3)
            .map(|i| {
                let c =
                    f64::from(u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex channel"))
                        / 255.0;
                if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
            })
            .collect()
    } else {
        // CSS Color 4 Oklab conversion, expressed directly in linear sRGB.
        // https://www.w3.org/TR/css-color-4/#color-conversion-code
        let components: Vec<f64> = value
            .strip_prefix("oklch(")
            .and_then(|v| v.strip_suffix(')'))
            .expect("expected hex or numeric OKLCH; extend parser for another export format")
            .split_whitespace()
            .map(|v| v.parse().expect("numeric OKLCH component"))
            .collect();
        assert_eq!(components.len(), 3, "opaque palette colors only");
        let (lightness, chroma, hue) = (components[0], components[1], components[2].to_radians());
        let (opponent_a, opponent_b) = (chroma * hue.cos(), chroma * hue.sin());
        let cone_l =
            (lightness + 0.396_337_777_4 * opponent_a + 0.215_803_757_3 * opponent_b).powi(3);
        let cone_m =
            (lightness - 0.105_561_345_8 * opponent_a - 0.063_854_172_8 * opponent_b).powi(3);
        let cone_s =
            (lightness - 0.089_484_177_5 * opponent_a - 1.291_485_548 * opponent_b).powi(3);
        vec![
            4.076_741_662_1 * cone_l - 3.307_711_591_3 * cone_m + 0.230_969_929_2 * cone_s,
            -1.268_438_004_6 * cone_l + 2.609_757_401_1 * cone_m - 0.341_319_396_5 * cone_s,
            -0.004_196_086_3 * cone_l - 0.703_418_614_7 * cone_m + 1.707_614_701 * cone_s,
        ]
    };
    // sRGB fallback contrast. Wide-gamut colors still require visual review.
    linear[0].clamp(0.0, 1.0) * 0.2126
        + linear[1].clamp(0.0, 1.0) * 0.7152
        + linear[2].clamp(0.0, 1.0) * 0.0722
}

#[test]
fn oklch_conversion_matches_srgb_reference_colors() {
    for (hex, oklch) in [
        ("#000000", "oklch(0 0 0)"),
        ("#ffffff", "oklch(1 0 0)"),
        ("#ff0000", "oklch(0.62795536 0.25768331 29.233885)"),
        ("#00ff00", "oklch(0.86643961 0.29482724 142.495339)"),
        ("#0000ff", "oklch(0.45201372 0.31321437 264.052021)"),
    ] {
        assert!((luminance(hex) - luminance(oklch)).abs() < 0.000_01, "{hex}");
    }
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
        if !colors.contains_key("action-ink") {
            colors.insert("action-ink", colors["primary"]);
        }
        for (fg, bg) in [
            ("foreground", "background"),
            ("card-foreground", "card"),
            ("popover-foreground", "popover"),
            ("muted-foreground", "background"),
            ("muted-foreground", "card"),
            ("muted-foreground", "popover"),
            ("primary-foreground", "primary"),
            ("action-ink", "background"),
            ("action-ink", "card"),
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

#[test]
fn published_values_are_preserved_in_both_modes() {
    let published: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/theme/published.json"))
            .expect("published registry snapshot");
    for (mode, selector) in [("light", ":root {"), ("dark", ".dark {")] {
        let actual = declarations(
            PRESET.split_once(selector).expect("selector").1.split('}').next().expect("block"),
        );
        let mut expected =
            published["cssVars"]["theme"].as_object().expect("common tokens").clone();
        expected.extend(published["cssVars"][mode].as_object().expect("mode tokens").clone());
        for (key, value) in expected {
            assert_eq!(actual.get(key.as_str()).copied(), value.as_str(), "{mode} {key}");
        }
    }
}
