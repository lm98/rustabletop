use serde::Serialize;

use crate::component::Rect;
use crate::impose::{Cell, Line, Sheet};

const FRAME: f64 = 7.0;
const MARK_LENGTH: f64 = 4.0;
const MARK_GAP: f64 = 1.0;
const HEADER: f64 = 16.0;
const FOOTER: f64 = 30.0;

#[derive(Debug, Clone, Copy)]
pub struct Board {
    pub trim: (f64, f64),
    pub bleed: f64,
}

impl Board {
    pub fn canvas(&self) -> (f64, f64) {
        (
            self.trim.0 + 2.0 * self.bleed,
            self.trim.1 + 2.0 * self.bleed,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileLayout {
    pub page: (f64, f64),
    pub margin: f64,
    pub overlap: f64,
    pub cols: Vec<(f64, f64)>,
    pub rows: Vec<(f64, f64)>,
}

impl TileLayout {
    pub fn new(board: &Board, paper: (f64, f64), margin: f64, overlap: f64) -> Option<Self> {
        let (w, h) = board.canvas();
        let layout = |page: (f64, f64)| {
            let window = |side: f64| side - 2.0 * (margin + FRAME);
            let (ww, wh) = (window(page.0), window(page.1));
            (ww > overlap && wh > overlap).then(|| Self {
                page,
                margin,
                overlap,
                cols: spans(w, ww, overlap),
                rows: spans(h, wh, overlap),
            })
        };
        let portrait = layout(paper);
        let landscape = layout((paper.1, paper.0));
        match (portrait, landscape) {
            (Some(p), Some(l)) if l.count() < p.count() => Some(l),
            (Some(p), _) => Some(p),
            (None, l) => l,
        }
    }

    pub fn count(&self) -> usize {
        self.cols.len() * self.rows.len()
    }

    fn origin(&self) -> (f64, f64) {
        (self.margin + FRAME, self.margin + FRAME)
    }

    pub fn window(&self, row: usize, col: usize) -> Rect {
        let (x, width) = self.cols[col];
        let (y, height) = self.rows[row];
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

pub fn spans(total: f64, window: f64, overlap: f64) -> Vec<(f64, f64)> {
    if total <= window + 1e-9 {
        return vec![(0.0, total)];
    }
    let stride = window - overlap;
    let count = ((total - overlap) / stride - 1e-9).ceil() as usize;
    (0..count)
        .map(|i| {
            let start = i as f64 * stride;
            (start, window.min(total - start))
        })
        .collect()
}

pub fn label(row: usize, col: usize) -> String {
    let mut letters = String::new();
    let mut n = row + 1;
    while n > 0 {
        let rem = (n - 1) % 26;
        letters.insert(0, char::from(b'A' + rem as u8));
        n = (n - 1) / 26;
    }
    format!("{letters}{}", col + 1)
}

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub component: usize,
    pub item: usize,
    pub page: (f64, f64),
    pub label: String,
    pub sections: (usize, usize),
    pub window: Rect,
    pub at: (f64, f64),
    pub under: Vec<Strip>,
    pub trims: Vec<Line>,
    pub marks: Vec<Line>,
    pub registration: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Strip {
    pub rect: Rect,
    pub label: String,
    pub vertical: bool,
}

pub fn tiles(component: usize, item: usize, board: &Board, layout: &TileLayout) -> Vec<Tile> {
    let (rows, cols) = (layout.rows.len(), layout.cols.len());
    let (ax, ay) = layout.origin();
    let (pw, ph) = layout.page;
    let mut tiles = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let window = layout.window(row, col);
            let to_page = |x: f64, y: f64| (ax + x - window.x, ay + y - window.y);
            let (right, bottom) = (window.x + window.width, window.y + window.height);

            let mut under = Vec::new();
            if col + 1 < cols {
                let next = layout.cols[col + 1].0;
                let (x, y) = to_page(next, window.y);
                under.push(Strip {
                    rect: Rect {
                        x,
                        y,
                        width: right - next,
                        height: window.height,
                    },
                    label: label(row, col + 1),
                    vertical: true,
                });
            }
            if row + 1 < rows {
                let next = layout.rows[row + 1].0;
                let (x, y) = to_page(window.x, next);
                under.push(Strip {
                    rect: Rect {
                        x,
                        y,
                        width: window.width,
                        height: bottom - next,
                    },
                    label: label(row + 1, col),
                    vertical: false,
                });
            }

            let mut trims = Vec::new();
            if col > 0 {
                trims.push(Line {
                    x1: ax,
                    y1: 0.0,
                    x2: ax,
                    y2: ph,
                });
            }
            if row > 0 {
                trims.push(Line {
                    x1: 0.0,
                    y1: ay,
                    x2: pw,
                    y2: ay,
                });
            }

            tiles.push(Tile {
                component,
                item,
                page: layout.page,
                label: label(row, col),
                sections: (rows, cols),
                window,
                at: (ax, ay),
                under,
                trims,
                marks: board_trim_marks(board, &window, (ax, ay)),
                registration: vec![
                    (ax, ay),
                    (ax + window.width, ay),
                    (ax, ay + window.height),
                    (ax + window.width, ay + window.height),
                ],
            });
        }
    }
    tiles
}

fn board_trim_marks(board: &Board, window: &Rect, (ax, ay): (f64, f64)) -> Vec<Line> {
    let (w, h) = board.canvas();
    let b = board.bleed;
    let inside = |v: f64, start: f64, len: f64| v >= start - 1e-9 && v <= start + len + 1e-9;
    let (top, bottom) = (ay - MARK_GAP, ay + window.height + MARK_GAP);
    let (left, right) = (ax - MARK_GAP, ax + window.width + MARK_GAP);
    let mut marks = Vec::new();
    for x in [b, w - b] {
        if inside(x, window.x, window.width) {
            let px = ax + x - window.x;
            marks.push(Line {
                x1: px,
                y1: top - MARK_LENGTH,
                x2: px,
                y2: top,
            });
            marks.push(Line {
                x1: px,
                y1: bottom,
                x2: px,
                y2: bottom + MARK_LENGTH,
            });
        }
    }
    for y in [b, h - b] {
        if inside(y, window.y, window.height) {
            let py = ay + y - window.y;
            marks.push(Line {
                x1: left - MARK_LENGTH,
                y1: py,
                x2: left,
                y2: py,
            });
            marks.push(Line {
                x1: right,
                y1: py,
                x2: right + MARK_LENGTH,
                y2: py,
            });
        }
    }
    marks
}

#[derive(Debug, Clone, Serialize)]
pub struct Assembly {
    pub component: usize,
    pub item: usize,
    pub page: (f64, f64),
    pub scale: f64,
    pub at: (f64, f64),
    pub canvas: (f64, f64),
    pub trim: (f64, f64),
    pub bleed: f64,
    pub overlap: f64,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub label: String,
    pub rect: Rect,
}

pub fn assembly(component: usize, item: usize, board: &Board, layout: &TileLayout) -> Assembly {
    let (pw, ph) = layout.page;
    let (w, h) = board.canvas();
    let m = layout.margin;
    let (aw, ah) = (pw - 2.0 * m, ph - 2.0 * m - HEADER - FOOTER);
    let scale = (aw / w).min(ah / h);
    let sections = (0..layout.rows.len())
        .flat_map(|row| (0..layout.cols.len()).map(move |col| (row, col)))
        .map(|(row, col)| Section {
            label: label(row, col),
            rect: layout.window(row, col),
        })
        .collect();
    Assembly {
        component,
        item,
        page: layout.page,
        scale,
        at: ((pw - w * scale) / 2.0, m + HEADER),
        canvas: (w, h),
        trim: board.trim,
        bleed: board.bleed,
        overlap: layout.overlap,
        sections,
    }
}

pub fn poster(
    component: usize,
    copies: &[usize],
    board: &Board,
    faces: &[&'static str],
) -> Vec<Sheet> {
    let b = board.bleed;
    copies
        .iter()
        .flat_map(|&item| {
            faces.iter().map(move |&face| Sheet {
                component,
                page: board.canvas(),
                cells: vec![Cell {
                    item,
                    face,
                    rotate: false,
                    x: b,
                    y: b,
                    l: b,
                    r: b,
                    t: b,
                    b,
                }],
                marks: Vec::new(),
                cut_lines: false,
                fold: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: (f64, f64) = (210.0, 297.0);

    fn dungeon() -> Board {
        Board {
            trim: (500.0, 350.0),
            bleed: 3.0,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn assert_covers(spans: &[(f64, f64)], total: f64, overlap: f64) {
        assert!(close(spans[0].0, 0.0));
        let last = spans.last().unwrap();
        assert!(close(last.0 + last.1, total));
        for pair in spans.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(b.0 > a.0);
            assert!(close(a.0 + a.1 - b.0, overlap), "{a:?} {b:?}");
        }
    }

    #[test]
    fn m2_6_exit_criterion_tiles_cover_the_whole_board_without_holes() {
        let board = dungeon();
        let layout = TileLayout::new(&board, A4, 5.0, 10.0).unwrap();
        assert_eq!(layout.page, (297.0, 210.0));
        assert_eq!((layout.rows.len(), layout.cols.len()), (2, 2));
        let (w, h) = board.canvas();
        assert_covers(&layout.cols, w, 10.0);
        assert_covers(&layout.rows, h, 10.0);

        let tiles = tiles(0, 0, &board, &layout);
        assert_eq!(tiles.len(), 4);
        let step = 0.5;
        let mut y = 0.0;
        while y <= h {
            let mut x = 0.0;
            while x <= w {
                let covered = tiles.iter().any(|t| {
                    let r = &t.window;
                    x >= r.x - 1e-9
                        && x <= r.x + r.width + 1e-9
                        && y >= r.y - 1e-9
                        && y <= r.y + r.height + 1e-9
                });
                assert!(covered, "({x}, {y}) is not covered");
                x += step;
            }
            y += step;
        }
        let max = (297.0 - 2.0 * (5.0 + FRAME), 210.0 - 2.0 * (5.0 + FRAME));
        assert!(
            tiles
                .iter()
                .all(|t| t.window.width <= max.0 + 1e-9 && t.window.height <= max.1 + 1e-9)
        );
    }

    #[test]
    fn strips_go_under_the_right_and_lower_neighbours() {
        let board = dungeon();
        let layout = TileLayout::new(&board, A4, 5.0, 10.0).unwrap();
        let tiles = tiles(0, 0, &board, &layout);
        let a1 = &tiles[0];
        assert_eq!(a1.label, "A1");
        let labels: Vec<&str> = a1.under.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["A2", "B1"]);
        assert!(close(a1.under[0].rect.width, 10.0));
        assert!(close(a1.under[0].rect.x + 10.0, a1.at.0 + a1.window.width));
        assert!(a1.trims.is_empty());
        let b2 = tiles.last().unwrap();
        assert_eq!(b2.label, "B2");
        assert_eq!(b2.trims.len(), 2);
        assert!(b2.under.is_empty());
        assert_eq!(a1.marks.len(), 4);
    }

    #[test]
    fn small_boards_fit_on_one_page() {
        let board = Board {
            trim: (150.0, 100.0),
            bleed: 3.0,
        };
        let layout = TileLayout::new(&board, A4, 5.0, 10.0).unwrap();
        assert_eq!(layout.count(), 1);
        let tiles = tiles(0, 0, &board, &layout);
        assert!(tiles[0].under.is_empty() && tiles[0].trims.is_empty());
        assert_eq!(tiles[0].marks.len(), 8);
    }

    #[test]
    fn labels_use_letters_for_rows() {
        assert_eq!(label(0, 0), "A1");
        assert_eq!(label(1, 2), "B3");
        assert_eq!(label(26, 0), "AA1");
    }

    #[test]
    fn m2_6_exit_criterion_poster_is_one_page_of_board_plus_bleed() {
        let board = dungeon();
        let sheets = poster(0, &[0], &board, &["front"]);
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].page, (506.0, 356.0));
        let cell = &sheets[0].cells[0];
        assert_eq!((cell.x, cell.y, cell.l), (3.0, 3.0, 3.0));
        assert_eq!(poster(0, &[0, 0], &board, &["front", "back"]).len(), 4);
    }

    #[test]
    fn assembly_thumbnail_fits_the_page() {
        let board = dungeon();
        let layout = TileLayout::new(&board, A4, 5.0, 10.0).unwrap();
        let a = assembly(0, 0, &board, &layout);
        assert_eq!(a.sections.len(), 4);
        assert!(a.at.0 >= 5.0 - 1e-9);
        assert!(a.at.0 + a.canvas.0 * a.scale <= 297.0 - 5.0 + 1e-9);
        assert!(a.at.1 + a.canvas.1 * a.scale <= 210.0 - 5.0 - FOOTER + 1e-9);
    }
}
