//! Renders the README animation: rows of icons tracing themselves in with
//! `Svg::drawn`, first from Lucide and then from Material.
//!
//! ```sh
//! cargo run --example readme -- /tmp/frames
//! convert -delay 3 -loop 0 /tmp/frames/*.svg -layers Optimize docs/draw-on.gif
//! ```

use std::{env, fs, path::PathBuf};

const W: f64 = 720.0;
const H: f64 = 260.0;
const COLORS: [&str; 4] = ["#6c8cff", "#ff8a65", "#2fbf8f", "#f2b632"];

const LUCIDE: [&str; 16] = [
    "house",
    "bell",
    "settings",
    "wifi",
    "battery-charging",
    "cloud-sun",
    "music",
    "camera",
    "heart",
    "message-circle",
    "calendar",
    "map-pin",
    "rocket",
    "shield-check",
    "zap",
    "cpu",
];
const MATERIAL: [&str; 16] = [
    "home",
    "notifications",
    "settings",
    "wifi",
    "battery-charging-full",
    "wb-sunny",
    "music-note",
    "photo-camera",
    "favorite",
    "chat",
    "event",
    "place",
    "rocket-launch",
    "verified-user",
    "bolt",
    "memory",
];

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn frame(names: &[&str], set: &str, t: f64, label: &str) -> String {
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}"><rect width="100%" height="100%" fill="#0f1420"/><text x="36" y="40" font-family="DejaVu Sans" font-size="15" fill="#8d97ad">{label}</text>"##
    );
    for (i, name) in names.iter().enumerate() {
        let icon = weaver_icons::get(&format!("{set}:{name}"))
            .unwrap_or_else(|| panic!("missing icon {set}:{name}"));
        let col = i % 8;
        let row = i / 8;
        let delay = (col + row * 2) as f64 * 0.06;
        let p = ease((t - delay) / 0.7);
        let x = 36.0 + col as f64 * 84.0;
        let y = 70.0 + row as f64 * 90.0;
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="64" height="64" rx="16" fill="#161d2c"/>"##
        ));
        svg.push_str(
            &icon
                .svg()
                .size(36.0)
                .color(COLORS[(col + row) % 4])
                .drawn(p)
                .group_at(x + 14.0, y + 14.0),
        );
    }
    svg.push_str("</svg>");
    svg
}

fn main() {
    let out = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "target/frames".into()));
    fs::create_dir_all(&out).expect("create output directory");
    let mut n = 0;
    for (names, set, label) in [
        (
            &LUCIDE,
            "lucide",
            "Lucide outline icons, traced with Svg::drawn",
        ),
        (
            &MATERIAL,
            "material",
            "Material filled icons: outline first, then the fill",
        ),
    ] {
        for f in 0..66 {
            let t = f as f64 / 30.0;
            fs::write(out.join(format!("{n:04}.svg")), frame(names, set, t, label)).unwrap();
            n += 1;
        }
    }
    println!("wrote {n} frames to {}", out.display());
}
