//! SVG output.

use std::fmt;

use crate::{Icon, Paint};

/// An SVG rendering of an icon. Configure it, then call `to_string()` (or
/// use it with `format!`).
///
/// ```
/// let icon = weaver_icons::get("bell").unwrap();
/// let full = icon.svg().size(48.0).color("tomato").title("Notifications").to_string();
/// let half_drawn = icon.svg().drawn(0.5).to_string();
/// let embedded = icon.svg().size(20.0).group_at(100.0, 40.0); // a <g> for a bigger SVG
/// # let _ = (full, half_drawn, embedded);
/// ```
#[derive(Clone, Debug)]
pub struct Svg {
    icon: Icon,
    size: f64,
    color: String,
    stroke_width: Option<f64>,
    drawn: f64,
    title: Option<String>,
}

impl Svg {
    pub(crate) fn new(icon: Icon) -> Self {
        Svg {
            icon,
            size: 24.0,
            color: "currentColor".into(),
            stroke_width: None,
            drawn: 1.0,
            title: None,
        }
    }

    /// Width and height in output units (default 24).
    pub fn size(mut self, size: f64) -> Self {
        self.size = size.max(0.0);
        self
    }

    /// Any CSS colour (default `currentColor`).
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Override the stroke width of outline icons (grid units, default 2).
    pub fn stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = Some(width.max(0.0));
        self
    }

    /// Draw-on progress from 0 (nothing) to 1 (complete, the default).
    /// Outlines are traced along their length; filled icons trace their
    /// outline first and then fade the fill in.
    pub fn drawn(mut self, progress: f64) -> Self {
        self.drawn = progress.clamp(0.0, 1.0);
        self
    }

    /// An accessible title (rendered as `<title>`).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    fn body(&self) -> String {
        let mut out = String::new();
        if let Some(title) = &self.title {
            out.push_str(&format!("<title>{}</title>", escape(title)));
        }
        let p = self.drawn;
        for d in self.icon.path_data() {
            match self.icon.paint() {
                Paint::Stroke { width } => {
                    let width = self.stroke_width.unwrap_or(width);
                    let dash = if p < 1.0 {
                        format!(
                            r#" pathLength="1" stroke-dasharray="1 1" stroke-dashoffset="{:.4}""#,
                            1.0 - p
                        )
                    } else {
                        String::new()
                    };
                    out.push_str(&format!(
                        r#"<path d="{d}" fill="none" stroke="{}" stroke-width="{width}" stroke-linecap="round" stroke-linejoin="round"{dash}/>"#,
                        escape(&self.color)
                    ));
                }
                Paint::Fill => {
                    if p >= 1.0 {
                        out.push_str(&format!(
                            r#"<path d="{d}" fill="{}"/>"#,
                            escape(&self.color)
                        ));
                    } else {
                        let trace = (p / 0.6).min(1.0);
                        let fill = ((p - 0.6) / 0.4).clamp(0.0, 1.0);
                        let fill = fill * fill * (3.0 - 2.0 * fill);
                        out.push_str(&format!(
                            r#"<path d="{d}" fill="{c}" fill-opacity="{fill:.4}" stroke="{c}" stroke-width="1" pathLength="1" stroke-dasharray="1 1" stroke-dashoffset="{:.4}"/>"#,
                            1.0 - trace,
                            c = escape(&self.color)
                        ));
                    }
                }
            }
        }
        out
    }

    /// A `<g>` element placing the icon at `(x, y)`, for embedding in a
    /// larger SVG.
    pub fn group_at(&self, x: f64, y: f64) -> String {
        format!(
            r#"<g transform="translate({x} {y}) scale({})">{}</g>"#,
            self.size / 24.0,
            self.body()
        )
    }
}

impl fmt::Display for Svg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role = if self.title.is_some() {
            r#" role="img""#
        } else {
            r#" aria-hidden="true""#
        };
        write!(
            f,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="0 0 24 24"{role}>{}</svg>"#,
            self.body(),
            s = self.size
        )
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
