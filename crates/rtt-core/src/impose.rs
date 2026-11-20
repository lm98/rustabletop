use serde::Serialize;

use crate::project::{Duplex, Print};
use crate::units::Size;

const MARK_LENGTH: f64 = 4.0;
const MARK_GAP: f64 = 1.0;
const MIN_MARK_LENGTH: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub x0: f64,
    pub y0: f64,
    pub cell: (f64, f64),
}

impl Grid {
    pub fn new(print: &Print, trim: Size, bleed: f64) -> Option<Self> {
        let paper = print.paper.size();
        let (pw, ph) = (paper.width.as_mm(), paper.height.as_mm());
        let (w, h) = (trim.width.as_mm(), trim.height.as_mm());
        let usable = |page: f64| page - 2.0 * print.margin.as_mm() - 2.0 * bleed;
        let fit = |space: f64, size: f64| (space / size + 1e-9).floor().max(0.0) as usize;
        let cols = fit(usable(pw), w);
        let rows = fit(usable(ph), h);
        (cols > 0 && rows > 0).then(|| Grid {
            cols,
            rows,
            x0: (pw - cols as f64 * w) / 2.0,
            y0: (ph - rows as f64 * h) / 2.0,
            cell: (w, h),
        })
    }

    pub fn per_sheet(&self) -> usize {
        self.cols * self.rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Front,
    Back,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    pub component: usize,
    pub face: Side,
    pub rotate: bool,
    pub cells: Vec<Cell>,
    pub marks: Vec<Mark>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    pub card: usize,
    pub x: f64,
    pub y: f64,
    pub l: f64,
    pub r: f64,
    pub t: f64,
    pub b: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mark {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

pub fn sheets(
    component: usize,
    copies: &[usize],
    grid: &Grid,
    bleed: f64,
    print: &Print,
    has_back: bool,
) -> Vec<Sheet> {
    let marks = if print.crop_marks {
        crop_marks(grid, bleed)
    } else {
        Vec::new()
    };
    let mut sheets = Vec::new();
    for chunk in copies.chunks(grid.per_sheet()) {
        let front = (0..chunk.len()).map(|i| slot(grid, i)).collect();
        sheets.push(Sheet {
            component,
            face: Side::Front,
            rotate: false,
            cells: place(grid, chunk, front, bleed),
            marks: marks.clone(),
        });
        if has_back && print.duplex != Duplex::None {
            let back = (0..chunk.len())
                .map(|i| mirror(grid, slot(grid, i), print.duplex))
                .collect();
            sheets.push(Sheet {
                component,
                face: Side::Back,
                rotate: print.duplex == Duplex::ShortEdge,
                cells: place(grid, chunk, back, bleed),
                marks: marks.clone(),
            });
        }
    }
    sheets
}

fn slot(grid: &Grid, index: usize) -> (usize, usize) {
    (index % grid.cols, index / grid.cols)
}

fn mirror(grid: &Grid, (col, row): (usize, usize), duplex: Duplex) -> (usize, usize) {
    match duplex {
        Duplex::LongEdge => (grid.cols - 1 - col, row),
        Duplex::ShortEdge => (col, grid.rows - 1 - row),
        Duplex::None => (col, row),
    }
}

fn place(grid: &Grid, cards: &[usize], slots: Vec<(usize, usize)>, bleed: f64) -> Vec<Cell> {
    let occupied = |col: Option<usize>, row: Option<usize>| match (col, row) {
        (Some(c), Some(r)) => slots.contains(&(c, r)),
        _ => false,
    };
    let edge = |neighbour: bool| if neighbour { 0.0 } else { bleed };
    cards
        .iter()
        .zip(&slots)
        .map(|(&card, &(col, row))| Cell {
            card,
            x: grid.x0 + col as f64 * grid.cell.0,
            y: grid.y0 + row as f64 * grid.cell.1,
            l: edge(occupied(col.checked_sub(1), Some(row))),
            r: edge(occupied(Some(col + 1), Some(row))),
            t: edge(occupied(Some(col), row.checked_sub(1))),
            b: edge(occupied(Some(col), Some(row + 1))),
        })
        .collect()
}

fn crop_marks(grid: &Grid, bleed: f64) -> Vec<Mark> {
    let gap = bleed + MARK_GAP;
    let length = (grid.x0.min(grid.y0) - gap).min(MARK_LENGTH);
    if length < MIN_MARK_LENGTH {
        return Vec::new();
    }
    let (w, h) = grid.cell;
    let (x0, y0) = (grid.x0, grid.y0);
    let (x1, y1) = (x0 + grid.cols as f64 * w, y0 + grid.rows as f64 * h);
    let mut marks = Vec::new();
    for i in 0..=grid.cols {
        let x = x0 + i as f64 * w;
        marks.push(Mark {
            x1: x,
            y1: y0 - gap - length,
            x2: x,
            y2: y0 - gap,
        });
        marks.push(Mark {
            x1: x,
            y1: y1 + gap,
            x2: x,
            y2: y1 + gap + length,
        });
    }
    for j in 0..=grid.rows {
        let y = y0 + j as f64 * h;
        marks.push(Mark {
            x1: x0 - gap - length,
            y1: y,
            x2: x0 - gap,
            y2: y,
        });
        marks.push(Mark {
            x1: x1 + gap,
            y1: y,
            x2: x1 + gap + length,
            y2: y,
        });
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Length;

    fn poker() -> Size {
        Size {
            width: Length::mm(63.0),
            height: Length::mm(88.0),
        }
    }

    #[test]
    fn poker_cards_fit_three_by_three_on_a4() {
        let grid = Grid::new(&Print::default(), poker(), 3.0).unwrap();
        assert_eq!((grid.cols, grid.rows), (3, 3));
        assert!((grid.x0 - 10.5).abs() < 1e-9);
        assert!((grid.y0 - 16.5).abs() < 1e-9);
    }

    #[test]
    fn oversized_cards_do_not_fit() {
        let huge = Size {
            width: Length::mm(150.0),
            height: Length::mm(300.0),
        };
        assert!(Grid::new(&Print::default(), huge, 3.0).is_none());
    }

    #[test]
    fn bleed_only_on_sides_without_neighbours() {
        let print = Print::default();
        let grid = Grid::new(&print, poker(), 3.0).unwrap();
        let sheets = sheets(0, &[0, 1, 2, 3], &grid, 3.0, &print, true);
        let front = &sheets[0].cells;
        assert_eq!(
            (front[0].l, front[0].r, front[0].t, front[0].b),
            (3.0, 0.0, 3.0, 0.0)
        );
        assert_eq!(
            (front[2].l, front[2].r, front[2].t, front[2].b),
            (0.0, 3.0, 3.0, 3.0)
        );
        assert_eq!(
            (front[3].l, front[3].r, front[3].t, front[3].b),
            (3.0, 3.0, 0.0, 3.0)
        );
    }

    #[test]
    fn long_edge_backs_are_mirrored_horizontally() {
        let print = Print::default();
        let grid = Grid::new(&print, poker(), 3.0).unwrap();
        let sheets = sheets(0, &[0, 1, 2, 3], &grid, 3.0, &print, true);
        assert_eq!(sheets.len(), 2);
        let (front, back) = (&sheets[0].cells, &sheets[1].cells);
        assert_eq!(back[0].x, front[2].x);
        assert_eq!(back[3].x, front[2].x);
        assert_eq!(back[3].y, front[3].y);
        assert_eq!((back[3].l, back[3].r), (3.0, 3.0));
        assert!(!sheets[1].rotate);
    }

    #[test]
    fn short_edge_backs_are_flipped_and_rotated() {
        let print = Print {
            duplex: Duplex::ShortEdge,
            ..Print::default()
        };
        let grid = Grid::new(&print, poker(), 3.0).unwrap();
        let sheets = sheets(0, &[0, 1], &grid, 3.0, &print, true);
        let (front, back) = (&sheets[0].cells, &sheets[1].cells);
        assert_eq!(back[0].x, front[0].x);
        assert!((back[0].y - (grid.y0 + 2.0 * 88.0)).abs() < 1e-9);
        assert!(sheets[1].rotate);
    }

    #[test]
    fn no_back_sheets_without_duplex_or_back_face() {
        let print = Print {
            duplex: Duplex::None,
            ..Print::default()
        };
        let grid = Grid::new(&print, poker(), 3.0).unwrap();
        assert_eq!(sheets(0, &[0; 10], &grid, 3.0, &print, true).len(), 2);
        assert_eq!(
            sheets(0, &[0; 10], &grid, 3.0, &Print::default(), false).len(),
            2
        );
        assert_eq!(
            sheets(0, &[0; 10], &grid, 3.0, &Print::default(), true).len(),
            4
        );
    }

    #[test]
    fn crop_marks_sit_outside_the_bleed() {
        let grid = Grid::new(&Print::default(), poker(), 3.0).unwrap();
        let marks = crop_marks(&grid, 3.0);
        assert_eq!(marks.len(), 2 * 4 + 2 * 4);
        assert!((marks[0].y2 - (grid.y0 - 4.0)).abs() < 1e-9);
    }
}
