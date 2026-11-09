//! SVG path data: parsing, normalising and flattening.
//!
//! [`parse`] reads the full SVG path grammar (absolute and relative
//! `M L H V C S Q T A Z`) and returns absolute commands with arcs and
//! quadratics converted to cubics, so a renderer only needs to support
//! move, line, cubic and close.

use std::f64::consts::PI;

/// One absolute drawing command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    /// Start a sub-path at `(x, y)`.
    MoveTo(f64, f64),
    /// Straight line to `(x, y)`.
    LineTo(f64, f64),
    /// Cubic Bézier with control points `(x1, y1)`, `(x2, y2)` to `(x, y)`.
    CubicTo(f64, f64, f64, f64, f64, f64),
    /// Close the sub-path.
    Close,
}

/// A path that could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset of the problem.
    pub offset: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid path data at byte {}", self.offset)
    }
}

impl std::error::Error for ParseError {}

struct Reader<'a> {
    s: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b',' | b'\n' | b'\t' | b'\r')
        {
            self.i += 1;
        }
    }

    fn command(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.i)?;
        if c.is_ascii_alphabetic() {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }

    fn at_number(&mut self) -> bool {
        self.skip();
        self.s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Result<f64, ParseError> {
        self.skip();
        let start = self.i;
        let s = self.s;
        if matches!(s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        let mut dot = false;
        while let Some(&c) = s.get(self.i) {
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == b'.' && !dot {
                dot = true;
                self.i += 1;
            } else {
                break;
            }
        }
        if matches!(s.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(s.get(self.i), Some(b'-' | b'+')) {
                self.i += 1;
            }
            while s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
        }
        std::str::from_utf8(&s[start..self.i])
            .ok()
            .and_then(|t| t.parse().ok())
            .ok_or(ParseError { offset: start })
    }

    /// Arc flags may be written without separators: `a1 1 0 001 1`.
    fn flag(&mut self) -> Result<bool, ParseError> {
        self.skip();
        match self.s.get(self.i) {
            Some(b'0') => {
                self.i += 1;
                Ok(false)
            }
            Some(b'1') => {
                self.i += 1;
                Ok(true)
            }
            _ => Err(ParseError { offset: self.i }),
        }
    }
}

/// Parse SVG path data into absolute move/line/cubic/close commands.
///
/// ```
/// use weaver_icons::path::{parse, PathCmd};
///
/// let cmds = parse("M2 2h10v10z").unwrap();
/// assert_eq!(cmds, vec![
///     PathCmd::MoveTo(2.0, 2.0),
///     PathCmd::LineTo(12.0, 2.0),
///     PathCmd::LineTo(12.0, 12.0),
///     PathCmd::Close,
/// ]);
/// ```
pub fn parse(d: &str) -> Result<Vec<PathCmd>, ParseError> {
    let mut r = Reader {
        s: d.as_bytes(),
        i: 0,
    };
    let mut out = Vec::new();
    let (mut x, mut y) = (0.0, 0.0);
    let (mut sx, mut sy) = (0.0, 0.0);
    // Last control point, for S and T reflections.
    let mut last_cubic: Option<(f64, f64)> = None;
    let mut last_quad: Option<(f64, f64)> = None;
    let mut cmd = match r.command() {
        Some(c) => c,
        None if r.s.iter().all(u8::is_ascii_whitespace) => return Ok(out),
        None => return Err(ParseError { offset: r.i }),
    };
    loop {
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (x, y) } else { (0.0, 0.0) };
        let mut next_cubic = None;
        let mut next_quad = None;
        match cmd.to_ascii_uppercase() {
            b'M' => {
                x = ox + r.number()?;
                y = oy + r.number()?;
                sx = x;
                sy = y;
                out.push(PathCmd::MoveTo(x, y));
                // Extra pairs after M are implicit L commands.
                cmd = if rel { b'l' } else { b'L' };
                if !r.at_number() {
                    if let Some(c) = r.command() {
                        cmd = c;
                        last_cubic = None;
                        last_quad = None;
                        continue;
                    }
                    break;
                }
                continue;
            }
            b'L' => {
                x = ox + r.number()?;
                y = oy + r.number()?;
                out.push(PathCmd::LineTo(x, y));
            }
            b'H' => {
                x = ox + r.number()?;
                out.push(PathCmd::LineTo(x, y));
            }
            b'V' => {
                y = oy + r.number()?;
                out.push(PathCmd::LineTo(x, y));
            }
            b'C' => {
                let x1 = ox + r.number()?;
                let y1 = oy + r.number()?;
                let x2 = ox + r.number()?;
                let y2 = oy + r.number()?;
                x = ox + r.number()?;
                y = oy + r.number()?;
                out.push(PathCmd::CubicTo(x1, y1, x2, y2, x, y));
                next_cubic = Some((x2, y2));
            }
            b'S' => {
                let (x1, y1) = last_cubic.map_or((x, y), |(cx, cy)| (2.0 * x - cx, 2.0 * y - cy));
                let x2 = ox + r.number()?;
                let y2 = oy + r.number()?;
                x = ox + r.number()?;
                y = oy + r.number()?;
                out.push(PathCmd::CubicTo(x1, y1, x2, y2, x, y));
                next_cubic = Some((x2, y2));
            }
            b'Q' => {
                let qx = ox + r.number()?;
                let qy = oy + r.number()?;
                let (nx, ny) = (ox + r.number()?, oy + r.number()?);
                out.push(quad_to_cubic(x, y, qx, qy, nx, ny));
                (x, y) = (nx, ny);
                next_quad = Some((qx, qy));
            }
            b'T' => {
                let (qx, qy) = last_quad.map_or((x, y), |(cx, cy)| (2.0 * x - cx, 2.0 * y - cy));
                let (nx, ny) = (ox + r.number()?, oy + r.number()?);
                out.push(quad_to_cubic(x, y, qx, qy, nx, ny));
                (x, y) = (nx, ny);
                next_quad = Some((qx, qy));
            }
            b'A' => {
                let rx = r.number()?.abs();
                let ry = r.number()?.abs();
                let rotation = r.number()?;
                let large = r.flag()?;
                let sweep = r.flag()?;
                let (nx, ny) = (ox + r.number()?, oy + r.number()?);
                arc_to_cubics(&mut out, (x, y), (rx, ry), rotation, large, sweep, (nx, ny));
                (x, y) = (nx, ny);
            }
            b'Z' => {
                out.push(PathCmd::Close);
                x = sx;
                y = sy;
            }
            _ => {
                return Err(ParseError {
                    offset: r.i.saturating_sub(1),
                })
            }
        }
        last_cubic = next_cubic;
        last_quad = next_quad;
        if cmd.eq_ignore_ascii_case(&b'Z') || !r.at_number() {
            match r.command() {
                Some(c) => cmd = c,
                None if r.i >= r.s.len() => break,
                None => {
                    r.skip();
                    if r.i >= r.s.len() {
                        break;
                    }
                    return Err(ParseError { offset: r.i });
                }
            }
        }
    }
    Ok(out)
}

fn quad_to_cubic(x0: f64, y0: f64, qx: f64, qy: f64, x: f64, y: f64) -> PathCmd {
    PathCmd::CubicTo(
        x0 + 2.0 / 3.0 * (qx - x0),
        y0 + 2.0 / 3.0 * (qy - y0),
        x + 2.0 / 3.0 * (qx - x),
        y + 2.0 / 3.0 * (qy - y),
        x,
        y,
    )
}

/// SVG endpoint arc to cubic Béziers (SVG 1.1 implementation notes, F.6).
fn arc_to_cubics(
    out: &mut Vec<PathCmd>,
    from: (f64, f64),
    radii: (f64, f64),
    rotation_deg: f64,
    large: bool,
    sweep: bool,
    to: (f64, f64),
) {
    let (x1, y1) = from;
    let (x2, y2) = to;
    let (mut rx, mut ry) = radii;
    if (x1 - x2).abs() < 1e-12 && (y1 - y2).abs() < 1e-12 {
        return;
    }
    if rx == 0.0 || ry == 0.0 {
        out.push(PathCmd::LineTo(x2, y2));
        return;
    }
    let phi = rotation_deg.to_radians();
    let (sin_p, cos_p) = phi.sin_cos();
    let dx = (x1 - x2) / 2.0;
    let dy = (y1 - y2) / 2.0;
    let x1p = cos_p * dx + sin_p * dy;
    let y1p = -sin_p * dx + cos_p * dy;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coef = (num / den).max(0.0).sqrt();
    if large == sweep {
        coef = -coef;
    }
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;
    let cx = cos_p * cxp - sin_p * cyp + (x1 + x2) / 2.0;
    let cy = sin_p * cxp + cos_p * cyp + (y1 + y2) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| {
        (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
    };
    let theta1 = angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut delta = angle(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }
    let segments = (delta.abs() / (PI / 2.0)).ceil().max(1.0) as usize;
    let step = delta / segments as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |t: f64| {
        let (s, c) = t.sin_cos();
        (
            cx + rx * c * cos_p - ry * s * sin_p,
            cy + rx * c * sin_p + ry * s * cos_p,
        )
    };
    let deriv = |t: f64| {
        let (s, c) = t.sin_cos();
        (
            -rx * s * cos_p - ry * c * sin_p,
            -rx * s * sin_p + ry * c * cos_p,
        )
    };
    for i in 0..segments {
        let t0 = theta1 + step * i as f64;
        let t1 = t0 + step;
        let (p0x, p0y) = point(t0);
        let (d0x, d0y) = deriv(t0);
        let (p1x, p1y) = if i + 1 == segments {
            (x2, y2)
        } else {
            point(t1)
        };
        let (d1x, d1y) = deriv(t1);
        out.push(PathCmd::CubicTo(
            p0x + k * d0x,
            p0y + k * d0y,
            p1x - k * d1x,
            p1y - k * d1y,
            p1x,
            p1y,
        ));
    }
}

/// Turn commands into polylines (one per sub-path), splitting curves until
/// they are within `tolerance` of the true shape.
pub fn flatten(cmds: &[PathCmd], tolerance: f64) -> Vec<Vec<(f64, f64)>> {
    let tolerance = tolerance.max(1e-4);
    let mut lines: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    let mut start = (0.0, 0.0);
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => {
                if current.len() > 1 {
                    lines.push(std::mem::take(&mut current));
                }
                current.clear();
                current.push((x, y));
                start = (x, y);
            }
            PathCmd::LineTo(x, y) => current.push((x, y)),
            PathCmd::CubicTo(x1, y1, x2, y2, x, y) => {
                let p0 = *current.last().unwrap_or(&start);
                let est = ((x1 - p0.0).hypot(y1 - p0.1)
                    + (x2 - x1).hypot(y2 - y1)
                    + (x - x2).hypot(y - y2))
                .max(1e-9);
                let n = ((est / tolerance).sqrt().ceil() as usize).clamp(1, 64);
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    let mt = 1.0 - t;
                    let a = mt * mt * mt;
                    let b = 3.0 * mt * mt * t;
                    let c = 3.0 * mt * t * t;
                    let d = t * t * t;
                    current.push((
                        a * p0.0 + b * x1 + c * x2 + d * x,
                        a * p0.1 + b * y1 + c * y2 + d * y,
                    ));
                }
            }
            PathCmd::Close => {
                current.push(start);
                if current.len() > 1 {
                    lines.push(std::mem::take(&mut current));
                }
                current.push(start);
            }
        }
    }
    if current.len() > 1 {
        lines.push(current);
    }
    lines
}

/// Approximate length of the outline.
pub fn length(cmds: &[PathCmd]) -> f64 {
    flatten(cmds, 0.05)
        .iter()
        .map(|line| {
            line.windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum::<f64>()
        })
        .sum()
}
