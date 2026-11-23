use serde::Serialize;

use crate::component::{Kind, Production, Sides};
use crate::project::{DuplexEdge, Print};
use crate::units::Size;

const MARK_LENGTH: f64 = 4.0;
const MARK_GAP: f64 = 1.0;
const MIN_MARK_LENGTH: f64 = 1.0;

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub kind: Kind,
    pub paper: (f64, f64),
    pub margin: f64,
    pub duplex_edge: DuplexEdge,
    pub trim: (f64, f64),
    pub bleed: f64,
    pub production: Production,
}

impl Layout {
    pub fn new(kind: Kind, print: &Print, trim: Size, bleed: f64, production: Production) -> Self {
        let paper = print.paper.size();
        Self {
            kind,
            paper: (paper.width.as_mm(), paper.height.as_mm()),
            margin: print.margin.as_mm(),
            duplex_edge: print.duplex_edge,
            trim: (trim.width.as_mm(), trim.height.as_mm()),
            bleed,
            production,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    pub component: usize,
    pub cells: Vec<Cell>,
    pub marks: Vec<Line>,
    pub cut_lines: bool,
    pub fold: Option<Line>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    pub item: usize,
    pub face: &'static str,
    pub rotate: bool,
    pub x: f64,
    pub y: f64,
    pub l: f64,
    pub r: f64,
    pub t: f64,
    pub b: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub x0: f64,
    pub y0: f64,
    pub pitch: (f64, f64),
}

impl Grid {
    pub fn per_sheet(&self) -> usize {
        self.cols * self.rows
    }

    fn slot(&self, index: usize) -> (usize, usize) {
        (index % self.cols, index / self.cols)
    }

    fn origin(&self, (col, row): (usize, usize)) -> (f64, f64) {
        (
            self.x0 + col as f64 * self.pitch.0,
            self.y0 + row as f64 * self.pitch.1,
        )
    }
}

fn fit(space: f64, cell: f64, gap: f64) -> usize {
    ((space + gap) / (cell + gap) + 1e-9).floor().max(0.0) as usize
}

pub fn grid(layout: &Layout) -> Option<Grid> {
    match layout.kind {
        Kind::Card => card_grid(layout),
        Kind::Token => token_grid(layout),
    }
}

fn card_grid(layout: &Layout) -> Option<Grid> {
    let (pw, ph) = layout.paper;
    let (w, h) = layout.trim;
    let usable = |page: f64| page - 2.0 * layout.margin - 2.0 * layout.bleed;
    let cols = fit(usable(pw), w, 0.0);
    let rows = fit(usable(ph), h, 0.0);
    (cols > 0 && rows > 0).then(|| Grid {
        cols,
        rows,
        x0: (pw - cols as f64 * w) / 2.0,
        y0: (ph - rows as f64 * h) / 2.0,
        pitch: (w, h),
    })
}

fn token_grid(layout: &Layout) -> Option<Grid> {
    let (pw, ph) = layout.paper;
    let (w, h) = layout.trim;
    let b = layout.bleed;
    let gap = layout.production.gap.as_mm();
    let (left, right) = match layout.production.sides {
        Sides::Fold => (layout.margin, pw / 2.0 - gap / 2.0),
        _ => (layout.margin, pw - layout.margin),
    };
    let (top, bottom) = (layout.margin, ph - layout.margin);
    let (bw, bh) = (w + 2.0 * b, h + 2.0 * b);
    let cols = fit(right - left, bw, gap);
    let rows = fit(bottom - top, bh, gap);
    let span = |n: usize, size: f64| n as f64 * size + n.saturating_sub(1) as f64 * gap;
    (cols > 0 && rows > 0).then(|| Grid {
        cols,
        rows,
        x0: left + (right - left - span(cols, bw)) / 2.0 + b,
        y0: top + (bottom - top - span(rows, bh)) / 2.0 + b,
        pitch: (bw + gap, bh + gap),
    })
}

pub fn sheets(component: usize, copies: &[usize], layout: &Layout, grid: &Grid) -> Vec<Sheet> {
    let marks = if layout.production.crop_marks {
        crop_marks(grid, layout)
    } else {
        Vec::new()
    };
    let mut sheets = Vec::new();
    for chunk in copies.chunks(grid.per_sheet()) {
        let fronts = match layout.kind {
            Kind::Card => card_cells(grid, chunk, layout.bleed),
            Kind::Token => token_cells(grid, chunk, layout.bleed),
        };
        let sheet = |cells: Vec<Cell>, fold: Option<Line>| Sheet {
            component,
            cells,
            marks: marks.clone(),
            cut_lines: layout.production.cut_lines,
            fold,
        };
        match layout.production.sides {
            Sides::Single => sheets.push(sheet(fronts, None)),
            Sides::Duplex => {
                let backs = fronts.iter().map(|c| back_duplex(c, layout)).collect();
                sheets.push(sheet(fronts, None));
                sheets.push(sheet(backs, None));
            }
            Sides::Fold => {
                let axis = layout.paper.0 / 2.0;
                let backs: Vec<Cell> = fronts
                    .iter()
                    .map(|c| mirror_x(c, axis, layout.trim.0))
                    .collect();
                let fold = Line {
                    x1: axis,
                    y1: layout.margin,
                    x2: axis,
                    y2: layout.paper.1 - layout.margin,
                };
                sheets.push(sheet(fronts.into_iter().chain(backs).collect(), Some(fold)));
            }
        }
    }
    sheets
}

fn card_cells(grid: &Grid, items: &[usize], bleed: f64) -> Vec<Cell> {
    let slots: Vec<(usize, usize)> = (0..items.len()).map(|i| grid.slot(i)).collect();
    let occupied = |col: Option<usize>, row: Option<usize>| match (col, row) {
        (Some(c), Some(r)) => slots.contains(&(c, r)),
        _ => false,
    };
    let edge = |neighbour: bool| if neighbour { 0.0 } else { bleed };
    items
        .iter()
        .zip(&slots)
        .map(|(&item, &(col, row))| {
            let (x, y) = grid.origin((col, row));
            Cell {
                item,
                face: "front",
                rotate: false,
                x,
                y,
                l: edge(occupied(col.checked_sub(1), Some(row))),
                r: edge(occupied(Some(col + 1), Some(row))),
                t: edge(occupied(Some(col), row.checked_sub(1))),
                b: edge(occupied(Some(col), Some(row + 1))),
            }
        })
        .collect()
}

fn token_cells(grid: &Grid, items: &[usize], bleed: f64) -> Vec<Cell> {
    items
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let (x, y) = grid.origin(grid.slot(i));
            Cell {
                item,
                face: "front",
                rotate: false,
                x,
                y,
                l: bleed,
                r: bleed,
                t: bleed,
                b: bleed,
            }
        })
        .collect()
}

fn mirror_x(cell: &Cell, axis: f64, width: f64) -> Cell {
    Cell {
        face: "back",
        x: 2.0 * axis - cell.x - width,
        l: cell.r,
        r: cell.l,
        ..cell.clone()
    }
}

fn back_duplex(cell: &Cell, layout: &Layout) -> Cell {
    let (pw, ph) = layout.paper;
    match layout.duplex_edge {
        DuplexEdge::Long => mirror_x(cell, pw / 2.0, layout.trim.0),
        DuplexEdge::Short => Cell {
            face: "back",
            rotate: true,
            y: ph - cell.y - layout.trim.1,
            t: cell.b,
            b: cell.t,
            ..cell.clone()
        },
    }
}

fn crop_marks(grid: &Grid, layout: &Layout) -> Vec<Line> {
    let gap = layout.bleed + MARK_GAP;
    let length = (grid.x0.min(grid.y0) - gap).min(MARK_LENGTH);
    if length < MIN_MARK_LENGTH {
        return Vec::new();
    }
    let (w, h) = grid.pitch;
    let (x0, y0) = (grid.x0, grid.y0);
    let (x1, y1) = (x0 + grid.cols as f64 * w, y0 + grid.rows as f64 * h);
    let mut marks = Vec::new();
    for i in 0..=grid.cols {
        let x = x0 + i as f64 * w;
        marks.push(Line {
            x1: x,
            y1: y0 - gap - length,
            x2: x,
            y2: y0 - gap,
        });
        marks.push(Line {
            x1: x,
            y1: y1 + gap,
            x2: x,
            y2: y1 + gap + length,
        });
    }
    for j in 0..=grid.rows {
        let y = y0 + j as f64 * h;
        marks.push(Line {
            x1: x0 - gap - length,
            y1: y,
            x2: x0 - gap,
            y2: y,
        });
        marks.push(Line {
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
    use crate::component::{self, Kind, ProductionDoc};
    use crate::units::Length;

    fn size(w: f64, h: f64) -> Size {
        Size {
            width: Length::mm(w),
            height: Length::mm(h),
        }
    }

    fn production(kind: Kind, sides: Option<Sides>) -> Production {
        let doc = ProductionDoc {
            sides,
            ..ProductionDoc::default()
        };
        component::production(kind, true, &doc).ok().unwrap()
    }

    fn card_layout(print: &Print) -> Layout {
        Layout::new(
            Kind::Card,
            print,
            size(63.0, 88.0),
            3.0,
            production(Kind::Card, None),
        )
    }

    fn token_layout(sides: Sides) -> Layout {
        Layout::new(
            Kind::Token,
            &Print::default(),
            size(25.0, 25.0),
            2.0,
            production(Kind::Token, Some(sides)),
        )
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn m2_5_exit_criterion_cards_keep_the_m2_layout() {
        let layout = card_layout(&Print::default());
        let grid = grid(&layout).unwrap();
        assert_eq!((grid.cols, grid.rows), (3, 3));
        assert!(close(grid.x0, 10.5) && close(grid.y0, 16.5));
        let sheets = sheets(0, &[0, 1, 2, 3], &layout, &grid);
        assert_eq!(sheets.len(), 2);
        let front: Vec<(f64, f64)> = sheets[0].cells.iter().map(|c| (c.x, c.y)).collect();
        assert_eq!(
            front,
            [(10.5, 16.5), (73.5, 16.5), (136.5, 16.5), (10.5, 104.5)]
        );
        let back: Vec<(f64, f64)> = sheets[1].cells.iter().map(|c| (c.x, c.y)).collect();
        assert_eq!(
            back,
            [(136.5, 16.5), (73.5, 16.5), (10.5, 16.5), (136.5, 104.5)]
        );
        assert_eq!(sheets[0].marks.len(), 16);
        assert!(!sheets[0].cut_lines);
    }

    #[test]
    fn card_bleed_only_on_sides_without_neighbours() {
        let layout = card_layout(&Print::default());
        let grid = grid(&layout).unwrap();
        let sheets = sheets(0, &[0, 1, 2, 3], &layout, &grid);
        let edges = |c: &Cell| (c.l, c.r, c.t, c.b);
        let (front, back) = (&sheets[0].cells, &sheets[1].cells);
        assert_eq!(edges(&front[0]), (3.0, 0.0, 3.0, 0.0));
        assert_eq!(edges(&front[2]), (0.0, 3.0, 3.0, 3.0));
        assert_eq!(edges(&front[3]), (3.0, 3.0, 0.0, 3.0));
        assert_eq!(edges(&back[0]), (0.0, 3.0, 3.0, 0.0));
        assert!(back.iter().all(|c| c.face == "back" && !c.rotate));
    }

    #[test]
    fn short_edge_backs_are_flipped_and_rotated() {
        let print = Print {
            duplex_edge: DuplexEdge::Short,
            ..Print::default()
        };
        let layout = card_layout(&print);
        let grid = grid(&layout).unwrap();
        let sheets = sheets(0, &[0, 1], &layout, &grid);
        let (front, back) = (&sheets[0].cells, &sheets[1].cells);
        assert_eq!(back[0].x, front[0].x);
        assert!(close(back[0].y, grid.y0 + 2.0 * 88.0));
        assert!(back[0].rotate);
    }

    #[test]
    fn oversized_components_do_not_fit() {
        let layout = Layout::new(
            Kind::Card,
            &Print::default(),
            size(150.0, 300.0),
            3.0,
            production(Kind::Card, None),
        );
        assert!(grid(&layout).is_none());
        let mut token = token_layout(Sides::Fold);
        token.trim = (110.0, 20.0);
        assert!(grid(&token).is_none());
    }

    #[test]
    fn tokens_are_spaced_and_bleed_on_every_side() {
        let layout = token_layout(Sides::Single);
        let grid = grid(&layout).unwrap();
        assert_eq!((grid.cols, grid.rows), (6, 9));
        let sheets = sheets(0, &[0, 1], &layout, &grid);
        let cells = &sheets[0].cells;
        assert!(close(cells[1].x - cells[0].x, 25.0 + 4.0 + 2.0));
        assert!(
            cells
                .iter()
                .all(|c| (c.l, c.r, c.t, c.b) == (2.0, 2.0, 2.0, 2.0))
        );
        assert!(sheets[0].cut_lines && sheets[0].marks.is_empty());
        assert!(cells[0].x - 2.0 >= 5.0 - 1e-9);
    }

    #[test]
    fn m2_5_exit_criterion_fold_backs_mirror_fronts_across_the_axis() {
        let layout = token_layout(Sides::Fold);
        let grid = grid(&layout).unwrap();
        let copies: Vec<usize> = (0..grid.per_sheet() + 3).collect();
        let sheets = sheets(0, &copies, &layout, &grid);
        assert_eq!(sheets.len(), 2);
        let axis = 105.0;
        for sheet in &sheets {
            let fold = sheet.fold.as_ref().unwrap();
            assert!(close(fold.x1, axis) && close(fold.x2, axis));
            let (fronts, backs): (Vec<&Cell>, Vec<&Cell>) =
                sheet.cells.iter().partition(|c| c.face == "front");
            assert_eq!(fronts.len(), backs.len());
            for front in &fronts {
                assert!(front.x + 25.0 + front.r <= axis);
                let back = backs
                    .iter()
                    .find(|b| b.item == front.item && close(b.y, front.y));
                let back = back.expect("every front has a back on the same row");
                assert!(close(axis - (front.x + 12.5), (back.x + 12.5) - axis));
                assert!(back.x - back.l >= axis);
            }
        }
    }

    #[test]
    fn duplex_tokens_use_the_whole_page() {
        let layout = token_layout(Sides::Duplex);
        let grid = grid(&layout).unwrap();
        let sheets = sheets(0, &[0], &layout, &grid);
        assert_eq!(sheets.len(), 2);
        assert!(close(
            sheets[1].cells[0].x,
            210.0 - sheets[0].cells[0].x - 25.0
        ));
        assert!(sheets.iter().all(|s| s.fold.is_none()));
    }
}
