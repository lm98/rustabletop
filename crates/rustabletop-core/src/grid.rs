use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::component::{Orientation, Shape};
use crate::units::{Length, Size};

const SQRT_3: f64 = 1.732_050_807_568_877_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GridType {
    Hex,
    Square,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridDoc {
    #[serde(rename = "type")]
    pub kind: GridType,
    pub cols: u32,
    pub rows: u32,
    pub cell: String,
    pub orientation: Option<Orientation>,
    pub origin: Option<[Length; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub kind: GridType,
    pub orientation: Orientation,
    pub cols: u32,
    pub rows: u32,
    pub cell: f64,
    pub origin: (f64, f64),
}

pub struct GridError {
    pub key: &'static str,
    pub message: String,
    pub hints: Vec<String>,
}

impl GridError {
    fn new(key: &'static str, message: impl Into<String>) -> Self {
        Self {
            key,
            message: message.into(),
            hints: Vec::new(),
        }
    }
}

pub fn resolve(
    doc: &GridDoc,
    board: Size,
    components: &[(String, Option<Shape>)],
) -> Result<Grid, GridError> {
    if doc.cols == 0 || doc.rows == 0 {
        let key = if doc.cols == 0 { "cols" } else { "rows" };
        return Err(GridError::new(key, format!("`{key}` must be at least 1")));
    }
    if doc.kind == GridType::Square && doc.orientation.is_some() {
        return Err(GridError::new(
            "orientation",
            "`orientation` only applies to `type: hex`",
        ));
    }
    let (cell, inherited) = cell_size(doc, components)?;
    if let (Some(declared), Some(inherited)) = (doc.orientation, inherited)
        && declared != inherited
    {
        return Err(GridError::new(
            "orientation",
            format!(
                "`orientation: {}` conflicts with the `{}` orientation of `{}`",
                orientation_name(declared),
                orientation_name(inherited),
                doc.cell
            ),
        ));
    }
    let mut grid = Grid {
        kind: doc.kind,
        orientation: doc.orientation.or(inherited).unwrap_or(Orientation::Pointy),
        cols: doc.cols,
        rows: doc.rows,
        cell,
        origin: (0.0, 0.0),
    };
    let (w, h) = (board.width.as_mm(), board.height.as_mm());
    grid.origin = match doc.origin {
        Some([x, y]) => (x.as_mm(), y.as_mm()),
        None => {
            let (x0, y0, x1, y1) = grid.extent();
            ((w - (x1 - x0)) / 2.0 - x0, (h - (y1 - y0)) / 2.0 - y0)
        }
    };
    let (x0, y0, x1, y1) = grid.extent();
    let eps = 1e-6;
    if x0 < -eps || y0 < -eps || x1 > w + eps || y1 > h + eps {
        return Err(GridError {
            key: if doc.origin.is_some() {
                "origin"
            } else {
                "cell"
            },
            message: format!(
                "the grid spans {:.1} × {:.1} mm from ({:.1}, {:.1}) and does not fit in the {w} × {h} mm board",
                x1 - x0,
                y1 - y0,
                x0,
                y0
            ),
            hints: vec!["reduce `cols`, `rows` or `cell`, or move `origin`".into()],
        });
    }
    Ok(grid)
}

fn orientation_name(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Pointy => "pointy",
        Orientation::Flat => "flat",
    }
}

fn cell_size(
    doc: &GridDoc,
    components: &[(String, Option<Shape>)],
) -> Result<(f64, Option<Orientation>), GridError> {
    if doc
        .cell
        .starts_with(|c: char| c.is_ascii_digit() || c == '.')
    {
        let length: Length = doc
            .cell
            .parse()
            .map_err(|m: String| GridError::new("cell", m))?;
        if length.as_mm() <= 0.0 {
            return Err(GridError::new("cell", "`cell` must be greater than zero"));
        }
        return Ok((length.as_mm(), None));
    }
    let Some((_, shape)) = components.iter().find(|(id, _)| *id == doc.cell) else {
        let ids: Vec<String> = components.iter().map(|(id, _)| id.clone()).collect();
        return Err(GridError {
            key: "cell",
            message: format!(
                "`cell: {}` is neither a length nor a component id",
                doc.cell
            ),
            hints: crate::schema::suggest(&doc.cell, &ids)
                .into_iter()
                .chain([format!("components: {}", ids.join(", "))])
                .collect(),
        });
    };
    let mismatch = |shape: &str| {
        let wanted = match doc.kind {
            GridType::Hex => "a `hex` token",
            GridType::Square => "a `square` or `circle` token",
        };
        GridError::new(
            "cell",
            format!(
                "`{}` is a {shape}, but a `{}` grid needs {wanted}",
                doc.cell,
                type_name(doc.kind)
            ),
        )
    };
    match (doc.kind, shape) {
        (_, None) => Err(GridError::new(
            "cell",
            format!("`{}` has an invalid shape", doc.cell),
        )),
        (
            GridType::Hex,
            Some(Shape::Hex {
                across_flats,
                orientation,
            }),
        ) => Ok((across_flats.as_mm(), Some(*orientation))),
        (GridType::Square, Some(Shape::Square { side, .. })) => Ok((side.as_mm(), None)),
        (GridType::Square, Some(Shape::Circle { diameter })) => Ok((diameter.as_mm(), None)),
        (_, Some(other)) => Err(mismatch(other.name())),
    }
}

fn type_name(kind: GridType) -> &'static str {
    match kind {
        GridType::Hex => "hex",
        GridType::Square => "square",
    }
}

impl Serialize for Grid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_json(0.0).serialize(serializer)
    }
}

impl Grid {
    pub fn cell_box(&self) -> (f64, f64) {
        let s = self.cell;
        match (self.kind, self.orientation) {
            (GridType::Square, _) => (s, s),
            (GridType::Hex, Orientation::Pointy) => (s, s * 2.0 / SQRT_3),
            (GridType::Hex, Orientation::Flat) => (s * 2.0 / SQRT_3, s),
        }
    }

    pub fn center(&self, col: u32, row: u32) -> (f64, f64) {
        let s = self.cell;
        let (c, r) = (f64::from(col), f64::from(row));
        let (dx, dy) = match (self.kind, self.orientation) {
            (GridType::Square, _) => (c * s, r * s),
            (GridType::Hex, Orientation::Pointy) => {
                let shift = if row % 2 == 1 { s / 2.0 } else { 0.0 };
                (c * s + shift, r * s * SQRT_3 / 2.0)
            }
            (GridType::Hex, Orientation::Flat) => {
                let shift = if col % 2 == 1 { s / 2.0 } else { 0.0 };
                (c * s * SQRT_3 / 2.0, r * s + shift)
            }
        };
        (self.origin.0 + dx, self.origin.1 + dy)
    }

    pub fn cells(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (0..self.rows).flat_map(move |row| (0..self.cols).map(move |col| (col, row)))
    }

    pub fn extent(&self) -> (f64, f64, f64, f64) {
        let (w, h) = self.cell_box();
        self.cells().map(|(c, r)| self.center(c, r)).fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(x0, y0, x1, y1), (x, y)| {
                (
                    x0.min(x - w / 2.0),
                    y0.min(y - h / 2.0),
                    x1.max(x + w / 2.0),
                    y1.max(y + h / 2.0),
                )
            },
        )
    }

    pub fn contains(&self, col: i64, row: i64) -> bool {
        (0..i64::from(self.cols)).contains(&col) && (0..i64::from(self.rows)).contains(&row)
    }

    pub fn describe_bounds(&self) -> String {
        format!(
            "grid is {} × {}: col 0..{}, row 0..{}",
            self.cols,
            self.rows,
            self.cols - 1,
            self.rows - 1
        )
    }

    pub fn to_json(&self, bleed: f64) -> Value {
        let (w, h) = self.cell_box();
        let centers: Vec<Value> = self
            .cells()
            .map(|(c, r)| {
                let (x, y) = self.center(c, r);
                json!([c, r, x + bleed, y + bleed])
            })
            .collect();
        json!({
            "type": type_name(self.kind),
            "orientation": orientation_name(self.orientation),
            "cols": self.cols,
            "rows": self.rows,
            "cell": self.cell,
            "width": w,
            "height": h,
            "origin": [self.origin.0 + bleed, self.origin.1 + bleed],
            "centers": centers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Size {
        Size {
            width: Length::mm(500.0),
            height: Length::mm(350.0),
        }
    }

    fn doc(kind: GridType, cell: &str) -> GridDoc {
        GridDoc {
            kind,
            cols: 14,
            rows: 9,
            cell: cell.into(),
            orientation: None,
            origin: None,
        }
    }

    fn vortex(orientation: Orientation) -> Vec<(String, Option<Shape>)> {
        vec![
            (
                "vortex".into(),
                Some(Shape::Hex {
                    across_flats: Length::mm(30.0),
                    orientation,
                }),
            ),
            (
                "seal".into(),
                Some(Shape::Circle {
                    diameter: Length::mm(25.0),
                }),
            ),
        ]
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    fn neighbours(grid: &Grid, col: u32, row: u32) -> Vec<(u32, u32)> {
        let (c, r) = (i64::from(col), i64::from(row));
        let offsets: [(i64, i64); 6] = match (grid.orientation, row % 2, col % 2) {
            (Orientation::Pointy, 0, _) => [(-1, 0), (1, 0), (-1, -1), (0, -1), (-1, 1), (0, 1)],
            (Orientation::Pointy, _, _) => [(-1, 0), (1, 0), (0, -1), (1, -1), (0, 1), (1, 1)],
            (Orientation::Flat, _, 0) => [(0, -1), (0, 1), (-1, -1), (-1, 0), (1, -1), (1, 0)],
            (Orientation::Flat, _, _) => [(0, -1), (0, 1), (-1, 0), (-1, 1), (1, 0), (1, 1)],
        };
        offsets
            .iter()
            .map(|(dc, dr)| (c + dc, r + dr))
            .filter(|(c, r)| grid.contains(*c, *r))
            .map(|(c, r)| (c as u32, r as u32))
            .collect()
    }

    #[test]
    fn m2_6_exit_criterion_hex_cells_match_the_vortex_token() {
        for orientation in [Orientation::Pointy, Orientation::Flat] {
            let grid = resolve(&doc(GridType::Hex, "vortex"), board(), &vortex(orientation))
                .ok()
                .unwrap();
            assert_eq!(grid.orientation, orientation);
            assert!(close(grid.cell, 30.0));
            let vortex_box = vortex(orientation)[0].1.unwrap().bbox();
            let (w, h) = grid.cell_box();
            assert!(close(w, vortex_box.width.as_mm()) && close(h, vortex_box.height.as_mm()));
            for (col, row) in grid.cells() {
                for (nc, nr) in neighbours(&grid, col, row) {
                    let d = distance(grid.center(col, row), grid.center(nc, nr));
                    assert!(close(d, 30.0), "({col},{row})-({nc},{nr}): {d}");
                }
            }
        }
    }

    #[test]
    fn square_grid_from_a_circle_token_and_default_centering() {
        let grid = resolve(
            &doc(GridType::Square, "seal"),
            board(),
            &vortex(Orientation::Pointy),
        )
        .ok()
        .unwrap();
        assert!(close(grid.cell, 25.0));
        assert!(close(distance(grid.center(0, 0), grid.center(1, 0)), 25.0));
        assert!(close(distance(grid.center(0, 0), grid.center(0, 1)), 25.0));
        let (x0, y0, x1, y1) = grid.extent();
        assert!(close(x0, 500.0 - x1) && close(y0, 350.0 - y1));
        assert!(close(x1 - x0, 14.0 * 25.0));
    }

    #[test]
    fn explicit_origin_is_the_center_of_the_first_cell() {
        let mut d = doc(GridType::Square, "20mm");
        d.origin = Some([Length::mm(25.0), Length::mm(30.0)]);
        let grid = resolve(&d, board(), &[]).ok().unwrap();
        assert_eq!(grid.center(0, 0), (25.0, 30.0));
        assert_eq!(grid.center(2, 1), (65.0, 50.0));
    }

    #[test]
    fn grid_errors_name_the_key() {
        let comps = vortex(Orientation::Pointy);
        let err = resolve(&doc(GridType::Hex, "vortx"), board(), &comps)
            .err()
            .unwrap();
        assert_eq!(err.key, "cell");
        assert_eq!(err.hints[0], "did you mean `vortex`?");
        assert_eq!(
            resolve(&doc(GridType::Hex, "seal"), board(), &comps)
                .err()
                .unwrap()
                .key,
            "cell"
        );
        let mut flat = doc(GridType::Hex, "vortex");
        flat.orientation = Some(Orientation::Flat);
        assert_eq!(
            resolve(&flat, board(), &comps).err().unwrap().key,
            "orientation"
        );
        let mut huge = doc(GridType::Square, "40mm");
        huge.cols = 20;
        assert_eq!(resolve(&huge, board(), &comps).err().unwrap().key, "cell");
        let mut outside = doc(GridType::Square, "20mm");
        outside.origin = Some([Length::mm(5.0), Length::mm(5.0)]);
        assert_eq!(
            resolve(&outside, board(), &comps).err().unwrap().key,
            "origin"
        );
    }
}
