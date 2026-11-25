use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use typst::foundations::{Label, Selector};
use typst::introspection::{Introspector, MetadataElem};
use typst::layout::PageRanges;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::board::{self, Board, TileLayout};
use crate::component::{BoardMode, Kind};
use crate::diagnostics::{Diagnostic, Subject};
use crate::impose::{self, Layout, Page, Sheet};
use crate::project::{self, Component, Instance, Project};
use crate::schema::{self, Format};
use crate::world::ProjectWorld;

const MAIN: &str = "/.rtt/main.typ";
const DATA: &str = "/.rtt/build.json";
const FACE_LABEL: &str = "rtt-face";
const OVERFLOW_LABEL: &str = "rtt-overflow";
const ART_LABEL: &str = "rtt-art";
const MIN_DPI: f64 = 200.0;

pub struct BuildOptions {
    pub out: PathBuf,
    pub ppi: f32,
    pub draft: bool,
}

pub struct RenderOptions {
    pub out: PathBuf,
    pub ppi: f32,
    pub face: Option<String>,
    pub draft: bool,
}

#[derive(Debug, Serialize)]
pub struct Built {
    pub ok: bool,
    pub artifacts: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn build(root: &Path, options: &BuildOptions) -> Result<Built> {
    let checked = project::check(root);
    let mut diagnostics = checked.diagnostics;
    let Some(project) = checked.project else {
        return Ok(Built {
            ok: false,
            artifacts: Vec::new(),
            diagnostics,
        });
    };

    let plan = match Plan::new(&project) {
        Ok(plan) => Plan {
            draft: options.draft,
            ..plan
        },
        Err(errors) => {
            diagnostics.extend(errors);
            return Ok(Built {
                ok: false,
                artifacts: Vec::new(),
                diagnostics,
            });
        }
    };

    let world = plan.world(&project)?;
    let document = match crate::compile(&world) {
        Ok(compiled) => {
            diagnostics.extend(compiled.warnings);
            compiled.document
        }
        Err(errors) => {
            diagnostics.extend(errors);
            return Ok(Built {
                ok: false,
                artifacts: Vec::new(),
                diagnostics,
            });
        }
    };
    diagnostics.extend(introspect(&project, &document));

    let sheets = plan.pages.len();
    let pdf = match pdf(&world, &document, sheets) {
        Ok(pdf) => pdf,
        Err(errors) => {
            diagnostics.extend(errors);
            return Ok(Built {
                ok: false,
                artifacts: Vec::new(),
                diagnostics,
            });
        }
    };

    let mut artifacts = Vec::new();
    fs::create_dir_all(&options.out)
        .with_context(|| format!("cannot create {}", options.out.display()))?;
    let suffix = if options.draft { "-draft" } else { "" };
    let pdf_path = options
        .out
        .join(format!("{}-pnp{suffix}.pdf", project.game));
    fs::write(&pdf_path, pdf)?;
    artifacts.push(pdf_path.display().to_string());
    artifacts.extend(write_previews(
        &project,
        &plan,
        &document,
        sheets,
        &options.out,
        options.ppi,
    )?);

    Ok(Built {
        ok: !diagnostics.iter().any(Diagnostic::is_error),
        artifacts,
        diagnostics,
    })
}

pub fn render(root: &Path, target: &str, options: &RenderOptions) -> Result<Built> {
    let inspected = project::inspect(root);
    let mut diagnostics = inspected.diagnostics;
    let failed = |diagnostics| Built {
        ok: false,
        artifacts: Vec::new(),
        diagnostics,
    };
    let Some(project) = inspected.project else {
        return Ok(failed(diagnostics));
    };
    let (index, item, faces) = match resolve_target(&project, target, options.face.as_deref()) {
        Ok(found) => found,
        Err(error) => {
            diagnostics.push(error);
            return Ok(failed(diagnostics));
        }
    };

    let mut plan = Plan::preview(&project, index, item, &faces);
    plan.draft = options.draft;
    plan.only = Some(index);
    let world = plan.world(&project)?;
    let document = match crate::compile(&world) {
        Ok(compiled) => {
            diagnostics.extend(compiled.warnings);
            compiled.document
        }
        Err(errors) => {
            diagnostics.extend(errors);
            return Ok(failed(diagnostics));
        }
    };
    diagnostics.extend(introspect(&project, &document));
    let artifacts = write_previews(&project, &plan, &document, 0, &options.out, options.ppi)?;
    Ok(Built {
        ok: !diagnostics.iter().any(Diagnostic::is_error),
        artifacts,
        diagnostics,
    })
}

fn resolve_target(
    project: &Project,
    target: &str,
    face: Option<&str>,
) -> Result<(usize, usize, Vec<&'static str>), Diagnostic> {
    let (component_id, instance_id) = match target.split_once('/') {
        Some((c, i)) => (c, Some(i)),
        None => (target, None),
    };
    let unknown = |message: String, hints: Vec<String>| {
        Diagnostic::error("render.target", message).hints(hints)
    };
    let ids: Vec<String> = project.components.iter().map(|c| c.id.clone()).collect();
    let Some(index) = project.components.iter().position(|c| c.id == component_id) else {
        return Err(unknown(
            format!("no component `{component_id}`"),
            schema::suggest(component_id, &ids)
                .into_iter()
                .chain([format!("components: {}", ids.join(", "))])
                .collect(),
        ));
    };
    let component = &project.components[index];
    let subject = |instance: Option<&str>| Subject {
        component: Some(component.id.clone()),
        instance: instance.map(str::to_owned),
        ..Subject::default()
    };
    let instances: Vec<String> = component.instances.iter().map(|i| i.id.clone()).collect();
    let item = match instance_id {
        None if instances.len() == 1 => 0,
        None => {
            return Err(unknown(
                format!(
                    "`{component_id}` has {} instances: name one",
                    instances.len()
                ),
                vec![format!(
                    "use `{component_id}/<instance>`, e.g. `{component_id}/{}`",
                    instances.first().map_or("…", String::as_str)
                )],
            )
            .about(subject(None)));
        }
        Some(id) => match instances.iter().position(|i| i == id) {
            Some(item) => item,
            None => {
                let listed: Vec<&str> = instances.iter().take(12).map(String::as_str).collect();
                let more = if instances.len() > listed.len() {
                    ", …"
                } else {
                    ""
                };
                return Err(unknown(
                    format!("`{component_id}` has no valid instance `{id}`"),
                    schema::suggest(id, &instances)
                        .into_iter()
                        .chain([format!("instances: {}{more}", listed.join(", "))])
                        .collect(),
                )
                .about(subject(Some(id))));
            }
        },
    };
    let available = component.face_names();
    let faces = match face {
        None => available,
        Some(face) => match available.iter().find(|f| **f == face) {
            Some(f) => vec![*f],
            None => {
                let names: Vec<String> = available.iter().map(|f| (*f).to_owned()).collect();
                return Err(unknown(
                    format!("`{component_id}` has no face `{face}`"),
                    schema::suggest(face, &names)
                        .into_iter()
                        .chain([format!("faces: {}", names.join(", "))])
                        .collect(),
                )
                .about(subject(Some(&component.instances[item].id))));
            }
        },
    };
    Ok((index, item, faces))
}

fn write_previews(
    project: &Project,
    plan: &Plan,
    document: &PagedDocument,
    skip: usize,
    out: &Path,
    ppi: f32,
) -> Result<Vec<String>> {
    let render = typst_render::RenderOptions {
        pixel_per_pt: f64::from(ppi / 72.0).into(),
        ..typst_render::RenderOptions::default()
    };
    let mut artifacts = Vec::new();
    for (page, preview) in document.pages().iter().skip(skip).zip(&plan.previews) {
        let component = &project.components[preview.component];
        let instance = &component.instances[preview.item];
        let dir = out.join(&component.id);
        fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
        let suffix = if plan.draft { "-draft" } else { "" };
        let path = dir.join(format!("{}-{}{suffix}.png", instance.id, preview.face));
        let png = typst_render::render(page, &render)
            .encode_png()
            .map_err(|e| anyhow::anyhow!("png encoding failed: {e}"))?;
        fs::write(&path, png)?;
        artifacts.push(path.display().to_string());
    }
    Ok(artifacts)
}

fn pdf(
    world: &ProjectWorld,
    document: &PagedDocument,
    sheets: usize,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let options = PdfOptions {
        page_ranges: Some(PageRanges::new(vec![
            NonZeroUsize::new(1)..=NonZeroUsize::new(sheets),
        ])),
        tagged: false,
        ..PdfOptions::default()
    };
    typst_pdf::pdf(document, &options).map_err(|errs| {
        errs.iter()
            .map(|d| Diagnostic::from_typst(world, d))
            .collect()
    })
}

#[derive(Debug, Serialize)]
struct Preview {
    component: usize,
    item: usize,
    face: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Plan {
    components: Vec<Value>,
    pages: Vec<Page>,
    previews: Vec<Preview>,
    draft: bool,
    #[serde(skip)]
    only: Option<usize>,
}

impl Plan {
    pub fn new(project: &Project) -> Result<Self, Vec<Diagnostic>> {
        let mut errors = Vec::new();
        let mut components = Vec::new();
        let mut pages = Vec::new();
        let mut previews = Vec::new();

        for (index, component) in project.components.iter().enumerate() {
            let bleed = component.bleed.as_mm();
            let bbox = component.shape.bbox();
            let copies: Vec<usize> = component
                .instances
                .iter()
                .enumerate()
                .flat_map(|(i, instance)| std::iter::repeat_n(i, instance.qty as usize))
                .collect();
            for item in 0..component.instances.len() {
                for face in component.face_names() {
                    previews.push(Preview {
                        component: index,
                        item,
                        face,
                    });
                }
            }
            components.push(component_data(component));

            if component.kind == Kind::Board {
                let board = Board {
                    trim: (bbox.width.as_mm(), bbox.height.as_mm()),
                    bleed,
                };
                if component.production.mode == Some(BoardMode::Poster) {
                    pages.extend(
                        board::poster(index, &copies, &board, &component.face_names())
                            .into_iter()
                            .map(Page::Sheet),
                    );
                    continue;
                }
                let paper = project.print.paper.size();
                let Some(layout) = TileLayout::new(
                    &board,
                    (paper.width.as_mm(), paper.height.as_mm()),
                    project.print.margin.as_mm(),
                    component.production.overlap.as_mm(),
                ) else {
                    errors.push(too_large(project, component));
                    continue;
                };
                let first = copies.first().copied().unwrap_or_default();
                pages.push(Page::Assembly(board::assembly(
                    index, first, &board, &layout,
                )));
                for &item in &copies {
                    pages.extend(
                        board::tiles(index, item, &board, &layout)
                            .into_iter()
                            .map(Page::Tile),
                    );
                }
                continue;
            }

            let layout = Layout::new(
                component.kind,
                &project.print,
                bbox,
                bleed,
                component.production,
            );
            let Some(grid) = impose::grid(&layout) else {
                errors.push(too_large(project, component));
                continue;
            };
            pages.extend(
                impose::sheets(index, &copies, &layout, &grid)
                    .into_iter()
                    .map(Page::Sheet),
            );
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Self {
            components,
            pages,
            previews,
            draft: false,
            only: None,
        })
    }

    fn preview(project: &Project, component: usize, item: usize, faces: &[&'static str]) -> Self {
        Self {
            components: project.components.iter().map(component_data).collect(),
            pages: Vec::new(),
            previews: faces
                .iter()
                .map(|face| Preview {
                    component,
                    item,
                    face,
                })
                .collect(),
            draft: false,
            only: None,
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn pages_of(&self, component: usize) -> Vec<&Page> {
        self.pages
            .iter()
            .filter(|p| p.component() == component)
            .collect()
    }

    pub fn sheets_of(&self, component: usize) -> Vec<&Sheet> {
        self.pages
            .iter()
            .filter_map(|p| match p {
                Page::Sheet(s) if s.component == component => Some(s),
                _ => None,
            })
            .collect()
    }

    pub fn preview_count(&self) -> usize {
        self.previews.len()
    }

    fn world(&self, project: &Project) -> Result<ProjectWorld> {
        let fonts: Vec<PathBuf> = project
            .fonts
            .iter()
            .map(|dir| project.root.join(dir))
            .collect();
        let mut world = ProjectWorld::virtual_main(&project.root, MAIN, &fonts)?;
        world.set_icons(&project.icons);
        world.set_draft(self.draft);
        world.add_file(MAIN, main_source(project, self.only))?;
        world.add_file(DATA, serde_json::to_vec(self)?)?;
        Ok(world)
    }
}

fn too_large(project: &Project, component: &Component) -> Diagnostic {
    let bbox = component.shape.bbox();
    Diagnostic::error(
        "print.too-large",
        format!(
            "a {} × {} {} with {} bleed does not fit on {:?} paper with {} margins",
            bbox.width,
            bbox.height,
            component.kind.name(),
            component.bleed,
            project.print.paper,
            project.print.margin
        ),
    )
    .about(Subject {
        component: Some(component.id.clone()),
        ..Subject::default()
    })
    .hint("use a larger paper, smaller margins or a smaller size")
}

fn component_data(component: &Component) -> Value {
    let properties = schema::property_names(&component.schema);
    let items: Vec<Value> = component
        .instances
        .iter()
        .map(|instance| {
            let mut item = Map::new();
            item.insert("id".into(), instance.id.clone().into());
            item.insert("qty".into(), instance.qty.into());
            for name in &properties {
                if name != "id" && name != "qty" {
                    let value = instance.fields.get(name).cloned().unwrap_or(Value::Null);
                    item.insert(name.clone(), value);
                }
            }
            Value::Object(item)
        })
        .collect();
    let assets: Vec<String> = schema::formatted_fields(&component.schema)
        .into_iter()
        .filter(|(_, format)| *format == Format::Asset)
        .map(|(name, _)| name)
        .collect();
    let bbox = component.shape.bbox();
    json!({
        "id": component.id,
        "kind": component.kind.name(),
        "shape": component.shape.to_json(),
        "trim": [bbox.width.as_mm(), bbox.height.as_mm()],
        "bleed": component.bleed.as_mm(),
        "grid": component.grid.as_ref().map(|g| g.to_json(component.bleed.as_mm())),
        "assets": assets,
        "items": items,
    })
}

fn main_source(project: &Project, only: Option<usize>) -> String {
    let mut src = String::new();
    let mut faces = String::new();
    for (i, component) in project.components.iter().enumerate() {
        let mut entry = String::new();
        let included = only.is_none_or(|o| o == i);
        for (face, path) in component.faces.iter().filter(|_| included) {
            let alias = format!("rtt-face-{i}-{face}");
            let _ = writeln!(src, "#import {:?}: {face} as {alias}", format!("/{path}"));
            let _ = write!(entry, "{face}: {alias}, ");
        }
        let _ = write!(
            faces,
            "({entry}{}), ",
            if entry.is_empty() { ":" } else { "" }
        );
    }
    let _ = writeln!(src, "#let rtt-faces = ({faces})");
    src.push_str(&MAIN_BODY.replace("{DATA}", DATA));
    src
}

const MAIN_BODY: &str = r##"#import "@rtt/std:0.1.0": outline
#let rtt-data = json("{DATA}")
#let rtt-mm(v) = v * 1mm
#let rtt-path(v) = if v == none { none } else if type(v) == array { v.map(rtt-path) } else { path("/" + v) }
#let rtt-note(size: 7pt, fill: black, body) = text(font: "DejaVu Sans Mono", size: size, fill: fill, body)

#let rtt-shape(s) = (
  type: s.type,
  orientation: s.orientation,
  width: rtt-mm(s.width),
  height: rtt-mm(s.height),
  radius: if s.radius == none { none } else { rtt-mm(s.radius) },
  inscribed: (
    x: rtt-mm(s.inscribed.at(0)), y: rtt-mm(s.inscribed.at(1)),
    width: rtt-mm(s.inscribed.at(2)), height: rtt-mm(s.inscribed.at(3)),
  ),
)

#let rtt-grid(g) = if g == none { none } else {
  (
    type: g.type,
    orientation: g.orientation,
    cols: g.cols,
    rows: g.rows,
    cell: rtt-mm(g.cell),
    width: rtt-mm(g.width),
    height: rtt-mm(g.height),
    origin: (rtt-mm(g.origin.at(0)), rtt-mm(g.origin.at(1))),
    centers: g.centers.map(((col, row, x, y)) => (col, row, rtt-mm(x), rtt-mm(y))),
  )
}

#let rtt-geometry(c) = (
  kind: c.kind,
  shape: rtt-shape(c.shape),
  trim: (width: rtt-mm(c.trim.at(0)), height: rtt-mm(c.trim.at(1))),
  bleed: rtt-mm(c.bleed),
)

#let rtt-components = rtt-data.components.map(c => {
  let items = c.items.map(item => {
    for field in c.assets { item.insert(field, rtt-path(item.at(field))) }
    item
  })
  (..c, items: items, geometry: rtt-geometry(c), grid: rtt-grid(c.grid))
})

#let rtt-all = {
  let all = (:)
  for c in rtt-components { all.insert(c.id, c.geometry) }
  all
}

#let rtt-ctx(c, face, item) = (
  ..c.geometry,
  component: c.id,
  face: face,
  item: item,
  grid: c.grid,
  components: rtt-all,
  draft: rtt-data.draft,
)

#let rtt-face(ci, k, face) = {
  let c = rtt-components.at(ci)
  let item = c.items.at(k)
  let ctx = rtt-ctx(c, face, item.id)
  box(width: ctx.trim.width + 2 * ctx.bleed, height: ctx.trim.height + 2 * ctx.bleed, clip: true, {
    place([#metadata((component: c.id, item: item.id, face: face)) <rtt-face>])
    rtt-faces.at(ci).at(face)(item, ctx)
  })
}

#let rtt-line(m, stroke) = place(top + left,
  line(start: (rtt-mm(m.x1), rtt-mm(m.y1)), end: (rtt-mm(m.x2), rtt-mm(m.y2)), stroke: stroke))

#let rtt-sheet(sheet) = {
  let c = rtt-components.at(sheet.component)
  let (w, h, b) = (c.trim.at(0), c.trim.at(1), c.bleed)
  page(width: rtt-mm(sheet.page.at(0)), height: rtt-mm(sheet.page.at(1)), margin: 0pt, {
    for m in sheet.marks { rtt-line(m, 0.25pt + black) }
    if sheet.fold != none {
      rtt-line(sheet.fold, (paint: luma(120), thickness: 0.4pt, dash: "dashed"))
    }
    for cell in sheet.cells {
      let face = rtt-face(sheet.component, cell.item, cell.face)
      if cell.rotate { face = rotate(180deg, face) }
      place(top + left, dx: rtt-mm(cell.x - cell.l), dy: rtt-mm(cell.y - cell.t),
        box(width: rtt-mm(w + cell.l + cell.r), height: rtt-mm(h + cell.t + cell.b), clip: true,
          place(top + left, dx: rtt-mm(cell.l - b), dy: rtt-mm(cell.t - b), face)))
    }
    if sheet.cut_lines {
      for cell in sheet.cells {
        for stroke in (1.2pt + white, 0.3pt + black) {
          place(top + left, dx: rtt-mm(cell.x - b), dy: rtt-mm(cell.y - b),
            outline(rtt-ctx(c, cell.face, none), fill: none, stroke: stroke))
        }
      }
    }
  })
}

#let rtt-tile(tile) = {
  let c = rtt-components.at(tile.component)
  let win = tile.window
  let (ax, ay) = (rtt-mm(tile.at.at(0)), rtt-mm(tile.at.at(1)))
  page(width: rtt-mm(tile.page.at(0)), height: rtt-mm(tile.page.at(1)), margin: 0pt, {
    place(top + left, dx: ax, dy: ay,
      box(width: rtt-mm(win.width), height: rtt-mm(win.height), clip: true,
        place(top + left, dx: -rtt-mm(win.x), dy: -rtt-mm(win.y), rtt-face(tile.component, tile.item, "front"))))
    for strip in tile.under {
      let r = strip.rect
      place(top + left, dx: rtt-mm(r.x), dy: rtt-mm(r.y),
        rect(width: rtt-mm(r.width), height: rtt-mm(r.height), stroke: none,
          fill: tiling(size: (3mm, 3mm), line(start: (0mm, 3mm), end: (3mm, 0mm), stroke: 0.3pt + luma(150)))))
      let note = box(fill: white, inset: 1mm, rtt-note(size: 6pt)[under #strip.label])
      place(top + left, dx: rtt-mm(r.x), dy: rtt-mm(r.y),
        box(width: rtt-mm(r.width), height: rtt-mm(r.height),
          align(center + horizon, if strip.vertical { rotate(-90deg, reflow: true, note) } else { note })))
    }
    for t in tile.trims {
      rtt-line(t, (paint: black, thickness: 0.4pt, dash: "dashed"))
      let vertical = t.x1 == t.x2
      place(top + left,
        dx: rtt-mm(t.x1) - if vertical { 1.6mm } else { -3mm },
        dy: rtt-mm(t.y1) - if vertical { -3mm } else { 1.9mm },
        rtt-note(size: 9pt, if vertical { rotate(90deg, reflow: true)[✂] } else { [✂] }))
    }
    for m in tile.marks { rtt-line(m, 0.25pt + black) }
    for (x, y) in tile.registration {
      let (x, y) = (rtt-mm(x), rtt-mm(y))
      let stroke = 0.25pt + black
      place(top + left, dx: x - 1.2mm, dy: y - 1.2mm, circle(radius: 1.2mm, stroke: stroke))
      place(top + left, dx: x - 2mm, dy: y, line(length: 4mm, stroke: stroke))
      place(top + left, dx: x, dy: y - 2mm, line(length: 4mm, angle: 90deg, stroke: stroke))
    }
    let (rows, cols) = (tile.sections.at(0), tile.sections.at(1))
    place(top + left, dx: ax + 4mm, dy: ay - 6mm,
      rtt-note(size: 8pt)[*#tile.label* #h(2mm) #c.id · section #tile.label of #(rows * cols) (#rows × #cols)])
  })
}

#let rtt-assembly(a) = {
  let c = rtt-components.at(a.component)
  let s = a.scale
  let (ax, ay) = (rtt-mm(a.at.at(0)), rtt-mm(a.at.at(1)))
  let (pw, ph) = (rtt-mm(a.page.at(0)), rtt-mm(a.page.at(1)))
  let mm-text(v) = str(calc.round(v, digits: 1))
  page(width: pw, height: ph, margin: 0pt, {
    place(top + left, dx: ax, dy: ay - 9mm, rtt-note(size: 11pt)[*#c.id* — assembly])
    place(top + left, dx: ax, dy: ay,
      box(width: rtt-mm(a.canvas.at(0) * s), height: rtt-mm(a.canvas.at(1) * s), clip: true,
        scale(x: s * 100%, y: s * 100%, origin: top + left, reflow: true,
          rtt-face(a.component, a.item, "front"))))
    for sec in a.sections {
      let r = sec.rect
      place(top + left, dx: ax + rtt-mm(r.x * s), dy: ay + rtt-mm(r.y * s),
        rect(width: rtt-mm(r.width * s), height: rtt-mm(r.height * s), stroke: 0.6pt + red, fill: none,
          align(center + horizon, box(fill: white, inset: 1mm, rtt-note(size: 10pt, fill: red)[*#sec.label*]))))
    }
    let bottom = ay + rtt-mm(a.canvas.at(1) * s) + 5mm
    place(top + left, dx: ax, dy: bottom, block(width: pw - 2 * ax, rtt-note(size: 8pt, {
      set par(spacing: 0.8em)
      [Final size: #mm-text(a.trim.at(0)) × #mm-text(a.trim.at(1)) mm, plus #mm-text(a.bleed) mm bleed. Sections overlap by #mm-text(a.overlap) mm.]
      enum(
        [Cut each section along the dashed ✂ lines on its left and top edges.],
        [Lay each section over the hatched strip marked "under ..." on its neighbour, aligning the registration marks.],
        [Glue, then trim the outer edges at the crop marks.],
      )
    })))
  })
}

#for p in rtt-data.pages {
  if p.type == "sheet" { rtt-sheet(p) }
  else if p.type == "tile" { rtt-tile(p) }
  else { rtt-assembly(p) }
}

#for p in rtt-data.previews {
  let c = rtt-components.at(p.component)
  page(width: rtt-mm(c.trim.at(0) + 2 * c.bleed), height: rtt-mm(c.trim.at(1) + 2 * c.bleed), margin: 0pt,
    rtt-face(p.component, p.item, p.face))
}
"##;

fn introspect(project: &Project, document: &PagedDocument) -> Vec<Diagnostic> {
    let label = |name: &str| Selector::Label(Label::new(PicoStr::intern(name)).expect("non-empty"));
    let selector = Selector::Or(
        [label(FACE_LABEL), label(OVERFLOW_LABEL), label(ART_LABEL)]
            .into_iter()
            .collect(),
    );
    let face_label = Label::new(PicoStr::intern(FACE_LABEL));
    let art_label = Label::new(PicoStr::intern(ART_LABEL));

    let mut current: Option<Face> = None;
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for element in document.introspector().query(&selector) {
        let Some(meta) = element.to_packed::<MetadataElem>() else {
            continue;
        };
        let value = serde_json::to_value(&meta.value).unwrap_or(Value::Null);
        if element.label() == face_label {
            current = Face::from_value(&value);
            continue;
        }
        let Some(face) = &current else { continue };
        let Some((component, instance)) = project
            .components
            .iter()
            .find(|c| c.id == face.component)
            .and_then(|c| Some((c, c.instances.iter().find(|i| i.id == face.item)?)))
        else {
            continue;
        };
        let diagnostic = if element.label() == art_label {
            low_resolution(project, component, instance, &value)
        } else {
            Some(overflow(face, instance, &value))
        };
        let Some(diagnostic) = diagnostic else {
            continue;
        };
        let field = diagnostic.subject.as_ref().and_then(|s| s.field.clone());
        let key = (
            diagnostic.code,
            face.component.clone(),
            face.item.clone(),
            face.face.clone(),
            field,
        );
        if seen.insert(key) {
            diagnostics.push(diagnostic);
        }
    }
    diagnostics
}

struct Face {
    component: String,
    item: String,
    face: String,
}

impl Face {
    fn from_value(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        Some(Self {
            component: text("component")?,
            item: text("item")?,
            face: text("face")?,
        })
    }
}

fn overflow(face: &Face, instance: &Instance, value: &Value) -> Diagnostic {
    let field = value
        .get("field")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let size = value
        .get("size")
        .and_then(Value::as_f64)
        .unwrap_or_default();
    let what = match &field {
        Some(f) => format!("`{f}`"),
        None => format!("a `fit` block on the {}", face.face),
    };
    let diagnostic = Diagnostic::warning(
        "render.overflow",
        format!(
            "{what} of {}/{} does not fit at {size}pt",
            face.component, face.item
        ),
    )
    .at(instance.location(field.as_deref()))
    .about(Subject {
        component: Some(face.component.clone()),
        instance: Some(face.item.clone()),
        field: field.clone(),
    })
    .hint(
        "shorten the text; lower `min` in the template's `fit` only if the size is still legible",
    );
    if field.is_none() {
        diagnostic.hint("pass `field: \"<name>\"` to `fit` to point at the field")
    } else {
        diagnostic
    }
}

fn low_resolution(
    project: &Project,
    component: &Component,
    instance: &Instance,
    value: &Value,
) -> Option<Diagnostic> {
    let source = value.get("source").and_then(Value::as_str)?;
    let rel = source
        .strip_prefix("path(\"")
        .and_then(|s| s.strip_suffix("\")"))
        .unwrap_or(source)
        .trim_start_matches('/');
    let extension = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg")) {
        return None;
    }
    let pixels = imagesize::size(project.root.join(rel)).ok()?;
    let inches = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0) / 72.0;
    let (width, height) = (inches("width"), inches("height"));
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let (x, y) = (pixels.width as f64 / width, pixels.height as f64 / height);
    let dpi = match value.get("fit").and_then(Value::as_str) {
        Some("contain") => x.max(y),
        _ => x.min(y),
    };
    if dpi >= MIN_DPI {
        return None;
    }
    let field = schema::formatted_fields(&component.schema)
        .into_iter()
        .filter(|(_, format)| *format == Format::Asset)
        .map(|(name, _)| name)
        .find(|name| instance.fields.get(name).and_then(Value::as_str) == Some(rel));
    let what = match &field {
        Some(f) => format!("`{f}`"),
        None => format!("`art` image `{rel}`"),
    };
    Some(
        Diagnostic::warning(
            "render.low-resolution",
            format!(
                "{what} of {}/{} prints at {} dpi (needs ≥ {MIN_DPI})",
                component.id,
                instance.id,
                dpi.floor()
            ),
        )
        .at(instance.location(field.as_deref()))
        .about(Subject {
            component: Some(component.id.clone()),
            instance: Some(instance.id.clone()),
            field,
        })
        .hint(format!(
            "at this size the image should be at least {} × {} px",
            (width * MIN_DPI).ceil(),
            (height * MIN_DPI).ceil()
        )),
    )
}
