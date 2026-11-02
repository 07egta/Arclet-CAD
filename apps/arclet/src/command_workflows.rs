//! Safe, opt-in typed CAD command workflows.
//!
//! The existing mouse tool remains the default for a bare command name.
//! Explicit coordinates or a JSON object run through the *same* engine registry;
//! no scripts, shell expressions, file URLs or implicit command guessing.
use serde_json::{Value, json};
use crate::commands;
pub(crate) const MAX_COMMAND_BYTES: usize = 8192;
pub(crate) const MAX_GUIDED_POINTS: usize = 4096;
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Invocation {
    Start(&'static str),
    Run { id: &'static str, params: Value },
    Guide(Guided),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuideKind { Line, Polyline, Circle, Box }
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Guided {
    pub(crate) kind: GuideKind,
    pub(crate) points: Vec<[f64; 2]>,
    pub(crate) lengths: Vec<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Advance {
    Waiting,
    Execute { id: &'static str, params: Value },
}
impl Guided {
    pub(crate) fn prompt(&self) -> String {
        match self.kind {
            GuideKind::Line => format!("Line — point {}: X,Y in mm (Esc cancels)", self.points.len() + 1),
            GuideKind::Polyline => format!("Polyline — {} point(s): X,Y; Enter completes; Esc cancels", self.points.len()),
            GuideKind::Circle => "Circle — radius (mm/cm/m/in/ft); Esc cancels".into(),
            GuideKind::Box => {
                let label = match self.lengths.len() { 0 => "length", 1 => "width", _ => "height" };
                format!("Box — {label} (mm/cm/m/in/ft); Esc cancels")
            }
        }
    }
    pub(crate) fn advance(&mut self, raw: &str) -> Result<Advance, String> {
        let raw = raw.trim();
        if raw.len() > MAX_COMMAND_BYTES || raw.chars().any(char::is_control) {
            return Err("Invalid command input".into());
        }
        match self.kind {
            GuideKind::Line | GuideKind::Polyline => {
                if raw.is_empty() {
                    if self.kind == GuideKind::Polyline && self.points.len() >= 2 {
                        return Ok(Advance::Execute { id: "sketch.line", params: json!({"points": self.points}) });
                    }
                    return Err("Pick at least two distinct points, or press Esc to cancel.".into());
                }
                if self.points.len() >= MAX_GUIDED_POINTS {
                    return Err("Too many points (limit 4096). Enter to finish or Esc to cancel.".into());
                }
                let p = parse_pair(raw)?;
                if self.points.last() == Some(&p) {
                    return Err("The next point must differ from the previous point.".into());
                }
                self.points.push(p);
                if self.kind == GuideKind::Line && self.points.len() == 2 {
                    Ok(Advance::Execute { id: "sketch.line", params: json!({"points": self.points}) })
                } else {
                    Ok(Advance::Waiting)
                }
            }
            GuideKind::Circle => {
                let radius = parse_length(raw)?;
                if radius <= 0.0 { return Err("Radius must be greater than zero.".into()); }
                let center = self.points.first().ok_or("Circle center was not specified.")?;
                Ok(Advance::Execute { id: "sketch.circle.center", params: json!({"center": center, "radius": as_mm(radius)}) })
            }
            GuideKind::Box => {
                let length = parse_length(raw)?;
                if length <= 0.0 { return Err("Each box dimension must be greater than zero.".into()); }
                self.lengths.push(length);
                if self.lengths.len() == 3 {
                    Ok(Advance::Execute { id: "solid.box", params: box_params(&self.lengths) })
                } else {
                    Ok(Advance::Waiting)
                }
            }
        }
    }
    /// Backspace in a guided multiline command removes a pending vertex/dimension,
    /// without touching document Undo or a separate viewport tool.
    pub(crate) fn back(&mut self) -> bool {
        match self.kind {
            GuideKind::Line | GuideKind::Polyline => {
                if self.points.len() > 1 { self.points.pop(); true } else { false }
            }
            GuideKind::Box => self.lengths.pop().is_some(),
            GuideKind::Circle => false,
        }
    }
}
fn as_mm(mm: f64) -> String {
    format!("{mm} mm")
}
fn box_params(values: &[f64]) -> Value {
    json!({
        "length": as_mm(values[0]),
        "width": as_mm(values[1]),
        "height": as_mm(values[2]),
    })
}
/// Coordinates are sketch-plane coordinates in mm, not arbitrary 3D world points.
pub(crate) fn parse_pair(raw: &str) -> Result<[f64; 2], String> {
    let (a, b) = raw.split_once(',').ok_or("Use X,Y, for example 20,35 or 2cm,35mm.")?;
    if b.contains(',') { return Err("A sketch point needs exactly two coordinates.".into()); }
    let x = parse_length(a.trim())?;
    let y = parse_length(b.trim())?;
    Ok([x, y])
}
/// A small, finite, absolute-length literal parser. Never evaluate arbitrary expressions.
/// Use explicit mm to make a typed input independent of the design's unit preference.
pub(crate) fn parse_length(raw: &str) -> Result<f64, String> {
    let t = raw.trim();
    if t.is_empty() || t.len() > 96 { return Err("Enter a finite number with optional mm, cm, m, in or ft.".into()); }
    let split = t.char_indices().find(|(_, ch)| !matches!(ch, '0'..='9' | '.' | '+' | '-' | 'e' | 'E')).map_or(t.len(), |(i, _)| i);
    let number = t[..split].parse::<f64>().map_err(|_| "Enter a valid number.")?;
    let scale = match t[split..].trim().to_ascii_lowercase().as_str() {
        "" | "mm" => 1.0,
        "cm" => 10.0,
        "m" => 1000.0,
        "in" => 25.4,
        "ft" => 304.8,
        _ => return Err("Unit must be mm, cm, m, in or ft.".into()),
    };
    let value = number * scale;
    if !value.is_finite() || value.abs() > 1e9 {
        return Err("Length must be finite and within 1,000,000,000 mm.".into());
    }
    Ok(value)
}
/// Parse a command name followed by *optional* explicit parameters. The original
/// text is not normalized after the name: quoted values preserve whitespace.
pub(crate) fn parse(text: &str) -> Result<Invocation, String> {
    if text.len() > MAX_COMMAND_BYTES || text.chars().any(char::is_control) {
        return Err("The command must be one line of at most 8192 bytes.".into());
    }
    let raw = text.trim();
    if raw.is_empty() { return Err("Enter a command name.".into()); }
    if let Ok(id) = commands::resolve(raw) { return Ok(Invocation::Start(id)); }
    let mut matched = None;
    for (at, c) in raw.char_indices() {
        if c.is_whitespace() {
            let first = raw[..at].trim();
            if let Ok(id) = commands::resolve(first) {
                matched = Some((id, first, raw[at..].trim()));
            }
        }
    }
    let Some((id, name, args)) = matched else {
        return commands::resolve(raw).map(Invocation::Start);
    };
    if args.is_empty() { return Ok(Invocation::Start(id)); }
    if args.starts_with('{') {
        let value: Value = serde_json::from_str(args).map_err(|e| format!("Invalid JSON parameters: {e}"))?;
        if !value.is_object() { return Err("Command parameters must be a JSON object.".into()); }
        return Ok(Invocation::Run { id, params: value });
    }
    let name = name.to_ascii_lowercase();
    match (id, name.as_str()) {
        ("sketch.line", "line" | "l" | "ln") => {
            let mut workflow = Guided { kind: GuideKind::Line, points: Vec::new(), lengths: Vec::new() };
            for token in args.split_whitespace() {
                if let Advance::Execute { id, params } = workflow.advance(token)? {
                    return Ok(Invocation::Run { id, params });
                }
            }
            Ok(Invocation::Guide(workflow))
        }
        ("sketch.line", "polyline") => {
            let mut workflow = Guided { kind: GuideKind::Polyline, points: Vec::new(), lengths: Vec::new() };
            for token in args.split_whitespace() { workflow.advance(token)?; }
            Ok(Invocation::Guide(workflow))
        }
        ("sketch.circle.center", "circle" | "c" | "ci") => {
            let mut parts = args.split_whitespace();
            let center = parse_pair(parts.next().ok_or("Circle center X,Y is required.")?)?;
            let workflow = Guided { kind: GuideKind::Circle, points: vec![center], lengths: Vec::new() };
            if let Some(radius) = parts.next() {
                if parts.next().is_some() { return Err("Circle accepts one center and one radius.".into()); }
                return match workflow.clone().advance(radius)? {
                    Advance::Execute { id, params } => Ok(Invocation::Run { id, params }),
                    Advance::Waiting => Ok(Invocation::Guide(workflow)),
                };
            }
            Ok(Invocation::Guide(workflow))
        }
        ("solid.box", "box" | "bx") => {
            let mut workflow = Guided { kind: GuideKind::Box, points: Vec::new(), lengths: Vec::new() };
            for token in args.split_whitespace() {
                if let Advance::Execute { id, params } = workflow.advance(token)? {
                    if workflow.lengths.len() != args.split_whitespace().count() { return Err("Box takes exactly three dimensions.".into()); }
                    return Ok(Invocation::Run { id, params });
                }
            }
            Ok(Invocation::Guide(workflow))
        }
        _ => Err("Use a JSON object after this command (e.g. Box {\"length\":\"20 mm\",\"width\":\"10 mm\",\"height\":\"5 mm\"}), or run its interactive tool by entering only its name.".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_numeric_lengths_are_converted_to_mm_and_reject_nonfinite() {
        assert_eq!(parse_length("2cm"), Ok(20.0));
        assert_eq!(parse_length("-1.5in"), Ok(-38.1));
        assert_eq!(parse_pair("2cm,3in"), Ok([20.0, 76.19999999999999]));
        for bad in ["", "NaN", "inf", "1e309", "2px", "10mm;", "2 3", "1,2,3", "1e8km"] {
            assert!(parse_length(bad).is_err(), "{bad}");
        }
        assert!(parse_pair("0,1,2").is_err());
    }
    #[test]
    fn single_line_commands_create_exact_engine_parameters() {
        assert_eq!(parse("LINE 0,0 2cm,10mm").unwrap(), Invocation::Run {
            id: "sketch.line", params: json!({"points":[[0,0],[20,10]]})
        });
        assert_eq!(parse("Circle 0,0 5mm").unwrap(), Invocation::Run {
            id: "sketch.circle.center", params: json!({"center":[0,0],"radius":"5 mm"})
        });
        assert_eq!(parse("BOX 10mm 20mm 30mm").unwrap(), Invocation::Run {
            id: "solid.box", params: json!({"length":"10 mm","width":"20 mm","height":"30 mm"})
        });
        assert_eq!(parse("Solid.Box {\"length\":\"20 mm\",\"width\":\"10 mm\",\"height\":\"5 mm\"}").unwrap(),
            Invocation::Run { id:"solid.box", params:json!({"length":"20 mm","width":"10 mm","height":"5 mm"}) });
    }
    #[test]
    fn typed_workflow_can_accumulate_cancel_and_finish_without_partial_edits() {
        let Invocation::Guide(mut line) = parse("line 0,0").unwrap() else { panic!("expected line guide") };
        assert_eq!(line.advance("2cm,0").unwrap(),
            Advance::Execute { id:"sketch.line", params:json!({"points":[[0,0],[20,0]]}) });
        let Invocation::Guide(mut polyline) = parse("Polyline 0,0 10,0").unwrap() else { panic!("expected polyline guide") };
        assert!(matches!(polyline.advance("10,10").unwrap(), Advance::Waiting));
        assert!(polyline.back());
        assert!(matches!(polyline.advance("10,10").unwrap(), Advance::Waiting));
        assert_eq!(polyline.advance("").unwrap(),
            Advance::Execute { id:"sketch.line", params:json!({"points":[[0,0],[10,0],[10,10]]}) });
        assert!(polyline.advance("10,10").is_err());
    }
    #[test]
    fn commands_never_guess_partial_names_or_execute_scripts() {
        for raw in ["extr 5mm", "line; box", "line 0,0 && box", "box $()", "solid.box [1,2,3]",
             "line\nbox", "line 0,0 0,0", "box 10mm 20mm 0mm", "circle 0,0 -2mm"] {
            assert!(parse(raw).is_err(), "{raw:?}");
        }
        assert!(parse(&"x".repeat(MAX_COMMAND_BYTES + 1)).is_err());
        assert!(matches!(parse("Loft"), Ok(Invocation::Start("surface.loft"))));
    }
}
