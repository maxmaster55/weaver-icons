//! # weaver-icons
//!
//! Over 4,000 icons for any Rust UI: the full [Lucide](https://lucide.dev)
//! outline set (ISC) and the filled [Material Icons](https://fonts.google.com/icons)
//! set (Apache-2.0), with search, SVG output, renderer-ready path commands
//! and a draw-on animation.
//!
//! ```
//! let icon = weaver_icons::get("house").unwrap();
//!
//! // An SVG string, sized and coloured.
//! let svg = icon.svg().size(32.0).color("#6c8cff").to_string();
//! assert!(svg.contains("<path"));
//!
//! // Or absolute move/line/cubic commands for your own renderer.
//! let commands = icon.commands();
//! # let _ = commands;
//!
//! // Find icons by name or tag.
//! let found: Vec<_> = weaver_icons::search("wifi").map(|i| i.name()).collect();
//! assert!(found.contains(&"wifi"));
//! ```
//!
//! Each set is a cargo feature (`lucide`, `material`, both on by default),
//! so you only compile the icons you use.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod path;
mod svg;

mod sets {
    #[cfg(feature = "lucide")]
    pub(crate) mod lucide;
    #[cfg(feature = "material")]
    pub(crate) mod material;
}

use path::PathCmd;
pub use svg::Svg;

/// Generated icon data.
#[doc(hidden)]
#[derive(Debug)]
pub struct IconData {
    pub(crate) name: &'static str,
    pub(crate) paths: &'static [&'static str],
    pub(crate) tags: &'static [&'static str],
}

/// Which icon family an icon comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSet {
    /// Lucide: 2px outlines on a 24×24 grid.
    Lucide,
    /// Material Icons: filled shapes on a 24×24 grid.
    Material,
}

impl IconSet {
    /// Every set compiled into this build.
    pub fn all() -> &'static [IconSet] {
        &[
            #[cfg(feature = "lucide")]
            IconSet::Lucide,
            #[cfg(feature = "material")]
            IconSet::Material,
        ]
    }

    /// The short name used in `"set:name"` lookups.
    pub fn prefix(self) -> &'static str {
        match self {
            IconSet::Lucide => "lucide",
            IconSet::Material => "material",
        }
    }

    fn data(self) -> &'static [IconData] {
        match self {
            #[cfg(feature = "lucide")]
            IconSet::Lucide => sets::lucide::ICONS,
            #[cfg(feature = "material")]
            IconSet::Material => sets::material::ICONS,
            #[allow(unreachable_patterns)]
            _ => &[],
        }
    }

    /// Look up an icon in this set by its exact name.
    pub fn get(self, name: &str) -> Option<Icon> {
        let data = self.data();
        data.binary_search_by(|d| d.name.cmp(name))
            .ok()
            .map(|i| Icon {
                data: &data[i],
                set: self,
            })
    }

    /// Every icon in this set, sorted by name.
    pub fn icons(self) -> impl Iterator<Item = Icon> {
        self.data().iter().map(move |data| Icon { data, set: self })
    }

    /// How the set is meant to be painted.
    pub fn paint(self) -> Paint {
        match self {
            IconSet::Lucide => Paint::Stroke { width: 2.0 },
            IconSet::Material => Paint::Fill,
        }
    }
}

/// How an icon's paths are painted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// Outline strokes of the given width (in 24×24 grid units).
    Stroke {
        /// Stroke width.
        width: f64,
    },
    /// Solid fills (non-zero winding).
    Fill,
}

/// One icon. Cheap to copy; all data is static.
#[derive(Clone, Copy)]
pub struct Icon {
    data: &'static IconData,
    set: IconSet,
}

impl std::fmt::Debug for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Icon({}:{})", self.set.prefix(), self.data.name)
    }
}

impl PartialEq for Icon {
    fn eq(&self, other: &Self) -> bool {
        self.set == other.set && self.data.name == other.data.name
    }
}

impl Eq for Icon {}

impl Icon {
    /// The icon's name, such as `"house"`.
    pub fn name(&self) -> &'static str {
        self.data.name
    }

    /// The set the icon belongs to.
    pub fn set(&self) -> IconSet {
        self.set
    }

    /// Search keywords (Lucide only).
    pub fn tags(&self) -> &'static [&'static str] {
        self.data.tags
    }

    /// How to paint it.
    pub fn paint(&self) -> Paint {
        self.set.paint()
    }

    /// The raw SVG path data strings, on a 24×24 grid.
    pub fn path_data(&self) -> &'static [&'static str] {
        self.data.paths
    }

    /// Absolute move/line/cubic/close commands on a 24×24 grid.
    pub fn commands(&self) -> Vec<PathCmd> {
        self.data
            .paths
            .iter()
            .flat_map(|d| path::parse(d).unwrap_or_default())
            .collect()
    }

    /// The commands scaled to `size × size` and moved to `(x, y)`.
    pub fn commands_at(&self, x: f64, y: f64, size: f64) -> Vec<PathCmd> {
        let s = size / 24.0;
        self.commands()
            .into_iter()
            .map(|c| match c {
                PathCmd::MoveTo(a, b) => PathCmd::MoveTo(x + a * s, y + b * s),
                PathCmd::LineTo(a, b) => PathCmd::LineTo(x + a * s, y + b * s),
                PathCmd::CubicTo(a, b, c, d, e, f) => PathCmd::CubicTo(
                    x + a * s,
                    y + b * s,
                    x + c * s,
                    y + d * s,
                    x + e * s,
                    y + f * s,
                ),
                PathCmd::Close => PathCmd::Close,
            })
            .collect()
    }

    /// Polylines approximating the icon within `tolerance` grid units.
    pub fn polylines(&self, tolerance: f64) -> Vec<Vec<(f64, f64)>> {
        path::flatten(&self.commands(), tolerance)
    }

    /// Total outline length in grid units (used by the draw-on effect).
    pub fn length(&self) -> f64 {
        path::length(&self.commands())
    }

    /// Start building an SVG for this icon.
    pub fn svg(&self) -> Svg {
        Svg::new(*self)
    }
}

/// Find an icon by name. Lucide is searched first, then Material. Use
/// `"material:home"` or `"lucide:house"` to pick a set explicitly.
pub fn get(name: &str) -> Option<Icon> {
    if let Some((prefix, rest)) = name.split_once(':') {
        return IconSet::all()
            .iter()
            .find(|s| s.prefix() == prefix)
            .and_then(|s| s.get(rest));
    }
    IconSet::all().iter().find_map(|s| s.get(name))
}

/// Every compiled-in icon.
pub fn all() -> impl Iterator<Item = Icon> {
    IconSet::all().iter().flat_map(|s| s.icons())
}

/// Icons whose name or tags contain `query` (case-insensitive), exact name
/// matches first.
pub fn search(query: &str) -> impl Iterator<Item = Icon> {
    let q = query.to_ascii_lowercase();
    let mut exact = Vec::new();
    let mut by_name = Vec::new();
    let mut by_tag = Vec::new();
    for icon in all() {
        if icon.name() == q {
            exact.push(icon);
        } else if icon.name().contains(&q) {
            by_name.push(icon);
        } else if icon.tags().iter().any(|t| t.contains(&q)) {
            by_tag.push(icon);
        }
    }
    exact.into_iter().chain(by_name).chain(by_tag)
}
