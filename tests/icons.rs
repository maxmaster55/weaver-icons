use weaver_icons::path::{flatten, parse, PathCmd};
use weaver_icons::{get, search, IconSet, Paint};

#[test]
fn every_icon_parses_and_stays_near_the_grid() {
    let mut count = 0;
    for icon in weaver_icons::all() {
        for d in icon.path_data() {
            let cmds = parse(d).unwrap_or_else(|e| panic!("{:?}: {e}: {d}", icon));
            assert!(!cmds.is_empty(), "{icon:?} has an empty path");
        }
        for line in icon.polylines(0.25) {
            for (x, y) in line {
                assert!(
                    (-1.0..=25.0).contains(&x) && (-1.0..=25.0).contains(&y),
                    "{icon:?} leaves the grid: {x},{y}"
                );
            }
        }
        count += 1;
    }
    assert!(count > 4000, "only {count} icons");
}

#[test]
fn lookups_by_name_and_set() {
    assert_eq!(get("house").unwrap().set(), IconSet::Lucide);
    assert_eq!(get("material:home").unwrap().set(), IconSet::Material);
    assert_eq!(get("material:home").unwrap().paint(), Paint::Fill);
    assert!(get("material:a-arrow-down").is_none());
    assert!(get("no-such-icon").is_none());
}

#[test]
fn search_puts_exact_matches_first() {
    let results: Vec<_> = search("wifi").collect();
    assert_eq!(results[0].name(), "wifi");
    assert!(results.iter().any(|i| i.name() == "wifi-off"));
    // Tag search: "home" is a tag of lucide's "house".
    assert!(search("home").any(|i| i.name() == "house"));
}

#[test]
fn relative_commands_arcs_and_shorthands_are_normalised() {
    let cmds = parse("m1 1 2 0v2h-2zM10 10q2 0 2 2t2 2s2 2 4 0a1 1 0 0 1 2 0").unwrap();
    assert_eq!(cmds[0], PathCmd::MoveTo(1.0, 1.0));
    assert_eq!(cmds[1], PathCmd::LineTo(3.0, 1.0));
    assert_eq!(cmds[2], PathCmd::LineTo(3.0, 3.0));
    assert_eq!(cmds[3], PathCmd::LineTo(1.0, 3.0));
    assert_eq!(cmds[4], PathCmd::Close);
    let PathCmd::CubicTo(.., x, y) = *cmds.last().unwrap() else {
        panic!("arc should end in a cubic");
    };
    assert!((x - 20.0).abs() < 1e-9 && (y - 14.0).abs() < 1e-9);
    // Compact arc flags: "a1 1 0 011 1".
    assert!(parse("M0 0a1 1 0 011 1").is_ok());
    assert!(parse("M0 0 L").is_err());
}

#[test]
fn a_circle_flattens_to_the_right_circumference() {
    let cmds = parse("M0 10a10 10 0 1 0 20 0a10 10 0 1 0-20 0z").unwrap();
    let len: f64 = flatten(&cmds, 0.01)
        .iter()
        .flat_map(|l| {
            l.windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        })
        .sum();
    assert!(
        (len - 2.0 * std::f64::consts::PI * 10.0).abs() < 0.2,
        "{len}"
    );
}

#[test]
fn svg_output_has_size_colour_and_draw_progress() {
    let icon = get("bell").unwrap();
    let svg = icon.svg().size(48.0).color("#123456").to_string();
    assert!(svg.contains(r#"width="48""#) && svg.contains("#123456"));
    assert!(svg.contains(r#"aria-hidden="true""#));
    assert!(!svg.contains("stroke-dashoffset"));
    let half = icon.svg().drawn(0.5).title("Bell <ring>").to_string();
    assert!(half.contains(r#"stroke-dashoffset="0.5000""#));
    assert!(half.contains("<title>Bell &lt;ring&gt;</title>"));
    let scaled = icon.commands_at(10.0, 10.0, 48.0);
    assert_eq!(scaled.len(), icon.commands().len());
    assert!(icon.length() > 10.0);
}
