use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::units::{Length, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Card,
    Token,
    Board,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardMode {
    Tiles,
    Poster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sides {
    Duplex,
    Fold,
    Single,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Card => "card",
            Kind::Token => "token",
            Kind::Board => "board",
        }
    }

    pub fn faces(self) -> &'static [&'static str] {
        &["front", "back"]
    }

    pub fn shapes(self) -> &'static [&'static str] {
        match self {
            Kind::Card | Kind::Board => &["rect"],
            Kind::Token => &["circle", "square", "hex"],
        }
    }

    pub fn sides(self) -> &'static [Sides] {
        match self {
            Kind::Card => &[Sides::Duplex, Sides::Single],
            Kind::Token => &[Sides::Fold, Sides::Duplex, Sides::Single],
            Kind::Board => &[],
        }
    }

    pub(crate) fn print_options(self) -> &'static [&'static str] {
        match self {
            Kind::Card => &["sides", "crop-marks"],
            Kind::Token => &["sides", "cut-lines", "gap"],
            Kind::Board => &["mode", "overlap"],
        }
    }

    pub fn default_gap(self) -> Length {
        match self {
            Kind::Card | Kind::Board => Length::mm(0.0),
            Kind::Token => Length::mm(2.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Pointy,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Rect {
        size: Size,
        radius: Length,
    },
    Circle {
        diameter: Length,
    },
    Square {
        side: Length,
        radius: Length,
    },
    Hex {
        across_flats: Length,
        orientation: Orientation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

const SQRT_3: f64 = 1.732_050_807_568_877_2;

impl Shape {
    pub fn name(&self) -> &'static str {
        match self {
            Shape::Rect { .. } => "rect",
            Shape::Circle { .. } => "circle",
            Shape::Square { .. } => "square",
            Shape::Hex { .. } => "hex",
        }
    }

    pub fn is_rectangular(&self) -> bool {
        matches!(self, Shape::Rect { .. } | Shape::Square { .. })
    }

    pub fn bbox(&self) -> Size {
        let (width, height) = match *self {
            Shape::Rect { size, .. } => (size.width.as_mm(), size.height.as_mm()),
            Shape::Circle { diameter } => (diameter.as_mm(), diameter.as_mm()),
            Shape::Square { side, .. } => (side.as_mm(), side.as_mm()),
            Shape::Hex {
                across_flats,
                orientation,
            } => {
                let flats = across_flats.as_mm();
                let corners = flats * 2.0 / SQRT_3;
                match orientation {
                    Orientation::Pointy => (flats, corners),
                    Orientation::Flat => (corners, flats),
                }
            }
        };
        Size {
            width: Length::mm(width),
            height: Length::mm(height),
        }
    }

    pub fn inscribed(&self) -> Rect {
        let bbox = self.bbox();
        let (w, h) = (bbox.width.as_mm(), bbox.height.as_mm());
        let (iw, ih) = match *self {
            Shape::Rect { .. } | Shape::Square { .. } => (w, h),
            Shape::Circle { diameter } => {
                let side = diameter.as_mm() / 2f64.sqrt();
                (side, side)
            }
            Shape::Hex {
                across_flats,
                orientation,
            } => {
                let flats = across_flats.as_mm();
                match orientation {
                    Orientation::Pointy => (flats, flats / SQRT_3),
                    Orientation::Flat => (flats / SQRT_3, flats),
                }
            }
        };
        Rect {
            x: (w - iw) / 2.0,
            y: (h - ih) / 2.0,
            width: iw,
            height: ih,
        }
    }

    pub fn to_json(&self) -> Value {
        let bbox = self.bbox();
        let (orientation, radius) = match *self {
            Shape::Rect { radius, .. } | Shape::Square { radius, .. } => {
                (Value::Null, Value::from(radius.as_mm()))
            }
            Shape::Circle { .. } => (Value::Null, Value::Null),
            Shape::Hex { orientation, .. } => (
                serde_json::to_value(orientation).unwrap_or(Value::Null),
                Value::Null,
            ),
        };
        let inscribed = self.inscribed();
        serde_json::json!({
            "type": self.name(),
            "orientation": orientation,
            "width": bbox.width.as_mm(),
            "height": bbox.height.as_mm(),
            "radius": radius,
            "inscribed": [inscribed.x, inscribed.y, inscribed.width, inscribed.height],
        })
    }
}

impl Serialize for Shape {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json().serialize(serializer)
    }
}

pub struct ShapeDoc<'a> {
    pub kind: Kind,
    pub shape: Option<&'a Value>,
    pub size: &'a Value,
    pub orientation: Option<Orientation>,
    pub radius: Option<Length>,
}

pub struct ShapeError {
    pub key: &'static str,
    pub message: String,
    pub hints: Vec<String>,
}

impl ShapeError {
    fn new(key: &'static str, message: impl Into<String>) -> Self {
        Self {
            key,
            message: message.into(),
            hints: Vec::new(),
        }
    }

    fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }
}

pub fn parse_shape(doc: &ShapeDoc) -> Result<Shape, ShapeError> {
    let allowed = doc.kind.shapes();
    let name = match doc.shape {
        None if allowed.len() == 1 => allowed[0],
        None => {
            return Err(
                ShapeError::new("shape", format!("a {} needs a `shape`", doc.kind.name()))
                    .hint(format!("shapes: {}", allowed.join(", "))),
            );
        }
        Some(Value::String(s)) => allowed.iter().find(|a| **a == s).copied().ok_or_else(|| {
            let candidates: Vec<String> = allowed.iter().map(|a| (*a).to_owned()).collect();
            ShapeError {
                key: "shape",
                message: format!("shape `{s}` is not available for a {}", doc.kind.name()),
                hints: crate::schema::suggest(s, &candidates)
                    .into_iter()
                    .chain([format!("shapes: {}", allowed.join(", "))])
                    .collect(),
            }
        })?,
        Some(_) => {
            return Err(ShapeError::new("shape", "`shape` must be a name")
                .hint(format!("shapes: {}", allowed.join(", "))));
        }
    };

    if doc.orientation.is_some() && name != "hex" {
        return Err(ShapeError::new(
            "orientation",
            "`orientation` only applies to `shape: hex`",
        ));
    }
    if doc.radius.is_some() && !matches!(name, "rect" | "square") {
        return Err(ShapeError::new(
            "radius",
            format!("`radius` does not apply to `shape: {name}`"),
        ));
    }
    let radius = doc.radius.unwrap_or(Length::mm(0.0));

    if name == "rect" {
        let size = Size::from_value(doc.size).map_err(|m| ShapeError::new("size", m))?;
        return Ok(Shape::Rect { size, radius });
    }
    let length = match doc.size {
        Value::String(s) => s
            .parse::<Length>()
            .map_err(|m| ShapeError::new("size", m))?,
        _ => {
            return Err(
                ShapeError::new("size", format!("a {name} `size` is a single length")).hint(
                    match name {
                        "circle" => "the diameter, e.g. `size: 25mm`",
                        "square" => "the side, e.g. `size: 25mm`",
                        _ => "the distance between opposite sides, e.g. `size: 30mm`",
                    },
                ),
            );
        }
    };
    if length.as_mm() <= 0.0 {
        return Err(ShapeError::new("size", "`size` must be greater than zero"));
    }
    if radius.as_mm() * 2.0 > length.as_mm() {
        return Err(ShapeError::new(
            "radius",
            "`radius` cannot exceed half the side",
        ));
    }
    Ok(match name {
        "circle" => Shape::Circle { diameter: length },
        "square" => Shape::Square {
            side: length,
            radius,
        },
        _ => Shape::Hex {
            across_flats: length,
            orientation: doc.orientation.unwrap_or(Orientation::Pointy),
        },
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ProductionDoc {
    pub sides: Option<Sides>,
    pub crop_marks: Option<bool>,
    pub cut_lines: Option<bool>,
    pub gap: Option<Length>,
    pub mode: Option<BoardMode>,
    pub overlap: Option<Length>,
}

impl ProductionDoc {
    fn set_options(&self) -> Vec<&'static str> {
        [
            ("sides", self.sides.is_some()),
            ("crop-marks", self.crop_marks.is_some()),
            ("cut-lines", self.cut_lines.is_some()),
            ("gap", self.gap.is_some()),
            ("mode", self.mode.is_some()),
            ("overlap", self.overlap.is_some()),
        ]
        .into_iter()
        .filter_map(|(key, set)| set.then_some(key))
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Production {
    pub sides: Sides,
    pub crop_marks: bool,
    pub cut_lines: bool,
    pub gap: Length,
    pub mode: Option<BoardMode>,
    pub overlap: Length,
}

const DEFAULT_OVERLAP: Length = Length::mm(10.0);
const MAX_OVERLAP_MM: f64 = 50.0;

pub fn production(
    kind: Kind,
    has_back: bool,
    doc: &ProductionDoc,
) -> Result<Production, ShapeError> {
    let sides_name = |s: Sides| {
        serde_json::to_value(s)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    let allowed = kind.print_options();
    if let Some(key) = doc.set_options().into_iter().find(|k| !allowed.contains(k)) {
        return Err(ShapeError::new(
            key,
            format!("`{key}` is not available for a {}", kind.name()),
        )
        .hint(format!(
            "{}s use {}",
            kind.name(),
            allowed
                .iter()
                .map(|k| format!("`{k}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if kind == Kind::Board {
        return board_production(has_back, doc);
    }
    if let Some(sides) = doc.sides
        && !kind.sides().contains(&sides)
    {
        let allowed: Vec<String> = kind.sides().iter().map(|s| sides_name(*s)).collect();
        return Err(ShapeError::new(
            "sides",
            format!(
                "`sides: {}` is not available for a {}",
                sides_name(sides),
                kind.name()
            ),
        )
        .hint(format!("sides: {}", allowed.join(", "))));
    }
    let sides = match (doc.sides, has_back) {
        (Some(Sides::Single), _) | (None, false) => Sides::Single,
        (Some(sides), false) => {
            return Err(ShapeError::new(
                "sides",
                format!("`sides: {}` needs a `back` face", sides_name(sides)),
            )
            .hint("add `back` to `faces`, or use `sides: single`"));
        }
        (Some(sides), true) => sides,
        (None, true) => kind.sides()[0],
    };
    Ok(Production {
        sides,
        crop_marks: kind == Kind::Card && doc.crop_marks.unwrap_or(true),
        cut_lines: kind == Kind::Token && doc.cut_lines.unwrap_or(true),
        gap: doc.gap.unwrap_or(kind.default_gap()),
        mode: None,
        overlap: Length::mm(0.0),
    })
}

fn board_production(has_back: bool, doc: &ProductionDoc) -> Result<Production, ShapeError> {
    let mode = doc.mode.unwrap_or(BoardMode::Tiles);
    if mode == BoardMode::Tiles && has_back {
        return Err(ShapeError::new(
            if doc.mode.is_some() { "mode" } else { "print" },
            "a board printed as `tiles` cannot have a `back` face",
        )
        .hint("use `mode: poster` to print a double-sided board"));
    }
    let overlap = doc.overlap.unwrap_or(DEFAULT_OVERLAP);
    if overlap.as_mm() > MAX_OVERLAP_MM {
        return Err(ShapeError::new(
            "overlap",
            format!("`overlap` must be at most {MAX_OVERLAP_MM}mm"),
        ));
    }
    Ok(Production {
        sides: if has_back {
            Sides::Duplex
        } else {
            Sides::Single
        },
        crop_marks: false,
        cut_lines: false,
        gap: Length::mm(0.0),
        mode: Some(mode),
        overlap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn token(shape: &str, size: &str, orientation: Option<Orientation>) -> Shape {
        parse_shape(&ShapeDoc {
            kind: Kind::Token,
            shape: Some(&json!(shape)),
            size: &json!(size),
            orientation,
            radius: None,
        })
        .ok()
        .unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn hex_bbox_follows_orientation() {
        let pointy = token("hex", "30mm", None).bbox();
        assert!(close(pointy.width.as_mm(), 30.0));
        assert!(close(pointy.height.as_mm(), 60.0 / SQRT_3));
        let flat = token("hex", "30mm", Some(Orientation::Flat)).bbox();
        assert!(close(flat.width.as_mm(), 60.0 / SQRT_3));
        assert!(close(flat.height.as_mm(), 30.0));
    }

    #[test]
    fn m2_5_exit_criterion_inscribed_rectangles_stay_inside_the_shape() {
        let circle = token("circle", "25mm", None).inscribed();
        let corner = ((circle.width / 2.0).powi(2) + (circle.height / 2.0).powi(2)).sqrt();
        assert!(close(corner, 12.5));
        assert!(close(circle.x, circle.y));

        let pointy = token("hex", "30mm", None);
        let r = pointy.inscribed();
        assert!(close(r.width, 30.0));
        let h = pointy.bbox().height.as_mm();
        assert!(close(r.y, h / 4.0));
        assert!(close(r.y + r.height, 3.0 * h / 4.0));

        let flat = token("hex", "30mm", Some(Orientation::Flat));
        let r = flat.inscribed();
        let w = flat.bbox().width.as_mm();
        assert!(close(r.height, 30.0));
        assert!(close(r.x, w / 4.0));

        let square = token("square", "20mm", None).inscribed();
        assert_eq!((square.x, square.width), (0.0, 20.0));
    }

    #[test]
    fn rejects_shapes_and_options_outside_the_kind() {
        let parse = |shape: Option<Value>, size: Value| {
            parse_shape(&ShapeDoc {
                kind: Kind::Token,
                shape: shape.as_ref(),
                size: &size,
                orientation: None,
                radius: None,
            })
            .err()
        };
        let err = parse(Some(json!("hexx")), json!("20mm")).unwrap();
        assert_eq!(err.key, "shape");
        assert_eq!(err.hints[0], "did you mean `hex`?");
        let err = parse(
            Some(json!("circle")),
            json!({"width": "1mm", "height": "1mm"}),
        )
        .unwrap();
        assert_eq!(err.key, "size");
        assert_eq!(parse(None, json!("20mm")).unwrap().key, "shape");
    }

    #[test]
    fn production_defaults_follow_kind_and_back() {
        let none = ProductionDoc::default();
        let token = production(Kind::Token, true, &none).ok().unwrap();
        assert_eq!(token.sides, Sides::Fold);
        assert!(token.cut_lines && !token.crop_marks);
        assert_eq!(token.gap.as_mm(), 2.0);
        assert_eq!(
            production(Kind::Card, true, &none).ok().unwrap().sides,
            Sides::Duplex
        );
        assert_eq!(
            production(Kind::Token, false, &none).ok().unwrap().sides,
            Sides::Single
        );

        let fold_card = ProductionDoc {
            sides: Some(Sides::Fold),
            ..ProductionDoc::default()
        };
        assert_eq!(
            production(Kind::Card, true, &fold_card).err().unwrap().key,
            "sides"
        );
        let duplex = ProductionDoc {
            sides: Some(Sides::Duplex),
            ..ProductionDoc::default()
        };
        assert_eq!(
            production(Kind::Token, false, &duplex).err().unwrap().key,
            "sides"
        );
        let marks_on_token = ProductionDoc {
            crop_marks: Some(true),
            ..ProductionDoc::default()
        };
        assert_eq!(
            production(Kind::Token, true, &marks_on_token)
                .err()
                .unwrap()
                .key,
            "crop-marks"
        );

        let board = production(Kind::Board, false, &none).ok().unwrap();
        assert_eq!(
            (board.mode, board.overlap.as_mm()),
            (Some(BoardMode::Tiles), 10.0)
        );
        let back_on_tiles = production(Kind::Board, true, &none).err().unwrap();
        assert_eq!(
            back_on_tiles.hints,
            ["use `mode: poster` to print a double-sided board"]
        );
        let poster = ProductionDoc {
            mode: Some(BoardMode::Poster),
            ..ProductionDoc::default()
        };
        let poster = production(Kind::Board, true, &poster).ok().unwrap();
        assert_eq!(poster.sides, Sides::Duplex);
        let sides_on_board = ProductionDoc {
            sides: Some(Sides::Single),
            ..ProductionDoc::default()
        };
        let err = production(Kind::Board, false, &sides_on_board)
            .err()
            .unwrap();
        assert_eq!(err.key, "sides");
        assert_eq!(err.hints, ["boards use `mode`, `overlap`"]);
    }
}
