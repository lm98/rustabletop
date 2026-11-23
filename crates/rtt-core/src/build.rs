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
use typst_render::RenderOptions;

use crate::diagnostics::{Diagnostic, Subject};
use crate::impose::{self, Layout, Sheet};
use crate::project::{self, Component, Project};
use crate::schema::{self, Format};
use crate::world::ProjectWorld;

const MAIN: &str = "/.rtt/main.typ";
const DATA: &str = "/.rtt/build.json";
const FACE_LABEL: &str = "rtt-face";
const OVERFLOW_LABEL: &str = "rtt-overflow";

pub struct BuildOptions {
    pub out: PathBuf,
    pub ppi: f32,
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
        Ok(plan) => plan,
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
    diagnostics.extend(overflows(&project, &document));

    let sheets = plan.sheets.len();
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
    let pdf_path = options.out.join(format!("{}-pnp.pdf", project.game));
    fs::write(&pdf_path, pdf)?;
    artifacts.push(pdf_path.display().to_string());

    let render = RenderOptions {
        pixel_per_pt: f64::from(options.ppi / 72.0).into(),
        ..RenderOptions::default()
    };
    for (page, preview) in document.pages().iter().skip(sheets).zip(&plan.previews) {
        let component = &project.components[preview.component];
        let instance = &component.instances[preview.item];
        let dir = options.out.join(&component.id);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-{}.png", instance.id, preview.face));
        let png = typst_render::render(page, &render)
            .encode_png()
            .map_err(|e| anyhow::anyhow!("png encoding failed: {e}"))?;
        fs::write(&path, png)?;
        artifacts.push(path.display().to_string());
    }

    Ok(Built {
        ok: !diagnostics.iter().any(Diagnostic::is_error),
        artifacts,
        diagnostics,
    })
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
    paper: (f64, f64),
    components: Vec<Value>,
    sheets: Vec<Sheet>,
    previews: Vec<Preview>,
}

impl Plan {
    pub fn new(project: &Project) -> Result<Self, Vec<Diagnostic>> {
        let paper = project.print.paper.size();
        let mut errors = Vec::new();
        let mut components = Vec::new();
        let mut sheets = Vec::new();
        let mut previews = Vec::new();

        for (index, component) in project.components.iter().enumerate() {
            let bleed = component.bleed.as_mm();
            let bbox = component.shape.bbox();
            let layout = Layout::new(
                component.kind,
                &project.print,
                bbox,
                bleed,
                component.production,
            );
            let Some(grid) = impose::grid(&layout) else {
                errors.push(
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
                    .hint("use a larger paper, smaller margins or a smaller size"),
                );
                continue;
            };
            let copies: Vec<usize> = component
                .instances
                .iter()
                .enumerate()
                .flat_map(|(i, instance)| std::iter::repeat_n(i, instance.qty as usize))
                .collect();
            sheets.extend(impose::sheets(index, &copies, &layout, &grid));
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
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Self {
            paper: (paper.width.as_mm(), paper.height.as_mm()),
            components,
            sheets,
            previews,
        })
    }

    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }

    pub fn sheets_of(&self, component: usize) -> Vec<&Sheet> {
        self.sheets
            .iter()
            .filter(|s| s.component == component)
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
        world.add_file(MAIN, main_source(project))?;
        world.add_file(DATA, serde_json::to_vec(self)?)?;
        Ok(world)
    }
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
        "assets": assets,
        "items": items,
    })
}

fn main_source(project: &Project) -> String {
    let mut src = String::new();
    let mut faces = String::new();
    for (i, component) in project.components.iter().enumerate() {
        let mut entry = String::new();
        for (face, path) in &component.faces {
            let alias = format!("rtt-face-{i}-{face}");
            let _ = writeln!(src, "#import {:?}: {face} as {alias}", format!("/{path}"));
            let _ = write!(entry, "{face}: {alias}, ");
        }
        let _ = write!(faces, "({entry}), ");
    }
    let _ = writeln!(src, "#let rtt-faces = ({faces})");
    src.push_str(&MAIN_BODY.replace("{DATA}", DATA));
    src
}

const MAIN_BODY: &str = r#"#import "@rtt/std:0.1.0": outline
#let rtt-data = json("{DATA}")
#let rtt-mm(v) = v * 1mm
#let rtt-path(v) = if v == none { none } else if type(v) == array { v.map(rtt-path) } else { path("/" + v) }
#let rtt-components = rtt-data.components.map(c => {
  let items = c.items.map(item => {
    for field in c.assets { item.insert(field, rtt-path(item.at(field))) }
    item
  })
  (..c, items: items)
})

#let rtt-ctx(c, face, item) = {
  let s = c.shape
  (
    kind: c.kind,
    shape: (
      type: s.type,
      orientation: s.orientation,
      width: rtt-mm(s.width),
      height: rtt-mm(s.height),
      radius: if s.radius == none { none } else { rtt-mm(s.radius) },
      inscribed: (
        x: rtt-mm(s.inscribed.at(0)), y: rtt-mm(s.inscribed.at(1)),
        width: rtt-mm(s.inscribed.at(2)), height: rtt-mm(s.inscribed.at(3)),
      ),
    ),
    trim: (width: rtt-mm(c.trim.at(0)), height: rtt-mm(c.trim.at(1))),
    bleed: rtt-mm(c.bleed),
    component: c.id,
    face: face,
    item: item,
  )
}

#let rtt-face(ci, k, face) = {
  let c = rtt-components.at(ci)
  let item = c.items.at(k)
  let ctx = rtt-ctx(c, face, item.id)
  box(width: ctx.trim.width + 2 * ctx.bleed, height: ctx.trim.height + 2 * ctx.bleed, clip: true, {
    place([#metadata((component: c.id, item: item.id, face: face)) <rtt-face>])
    rtt-faces.at(ci).at(face)(item, ctx)
  })
}

#let rtt-paper = (width: rtt-mm(rtt-data.paper.at(0)), height: rtt-mm(rtt-data.paper.at(1)))

#for sheet in rtt-data.sheets {
  let c = rtt-components.at(sheet.component)
  let (w, h, b) = (c.trim.at(0), c.trim.at(1), c.bleed)
  page(width: rtt-paper.width, height: rtt-paper.height, margin: 0pt, {
    for m in sheet.marks {
      place(top + left, line(start: (rtt-mm(m.x1), rtt-mm(m.y1)), end: (rtt-mm(m.x2), rtt-mm(m.y2)), stroke: 0.25pt + black))
    }
    if sheet.fold != none {
      let f = sheet.fold
      place(top + left, line(start: (rtt-mm(f.x1), rtt-mm(f.y1)), end: (rtt-mm(f.x2), rtt-mm(f.y2)),
        stroke: (paint: luma(120), thickness: 0.4pt, dash: "dashed")))
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
        place(top + left, dx: rtt-mm(cell.x - b), dy: rtt-mm(cell.y - b),
          outline(rtt-ctx(c, cell.face, none), fill: none, stroke: 0.3pt + luma(60)))
      }
    }
  })
}

#for p in rtt-data.previews {
  let c = rtt-components.at(p.component)
  page(width: rtt-mm(c.trim.at(0) + 2 * c.bleed), height: rtt-mm(c.trim.at(1) + 2 * c.bleed), margin: 0pt,
    rtt-face(p.component, p.item, p.face))
}
"#;

fn overflows(project: &Project, document: &PagedDocument) -> Vec<Diagnostic> {
    let label = |name: &str| Selector::Label(Label::new(PicoStr::intern(name)).expect("non-empty"));
    let selector = Selector::Or(
        [label(FACE_LABEL), label(OVERFLOW_LABEL)]
            .into_iter()
            .collect(),
    );
    let face_label = Label::new(PicoStr::intern(FACE_LABEL));

    let mut current: Option<Value> = None;
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for element in document.introspector().query(&selector) {
        let Some(meta) = element.to_packed::<MetadataElem>() else {
            continue;
        };
        let value = serde_json::to_value(&meta.value).unwrap_or(Value::Null);
        if element.label() == face_label {
            current = Some(value);
            continue;
        }
        let Some(face) = &current else { continue };
        let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
        let (Some(component), Some(item), Some(side)) = (
            text(face, "component"),
            text(face, "item"),
            text(face, "face"),
        ) else {
            continue;
        };
        let field = text(&value, "field");
        let size = value
            .get("size")
            .and_then(Value::as_f64)
            .unwrap_or_default();
        if !seen.insert((component.clone(), item.clone(), side.clone(), field.clone())) {
            continue;
        }
        let instance = project
            .components
            .iter()
            .find(|c| c.id == component)
            .and_then(|c| c.instances.iter().find(|i| i.id == item));
        let what = match &field {
            Some(f) => format!("`{f}`"),
            None => format!("a `fit` block on the {side}"),
        };
        let mut diagnostic = Diagnostic::warning(
            "render.overflow",
            format!("{what} of {component}/{item} does not fit at {size}pt"),
        )
        .at(instance.and_then(|i| i.location(field.as_deref())))
        .about(Subject {
            component: Some(component),
            instance: Some(item),
            field: field.clone(),
        })
        .hint("shorten the text, or lower `min` in the template's `fit`");
        if field.is_none() {
            diagnostic = diagnostic.hint("pass `field: \"<name>\"` to `fit` to point at the field");
        }
        diagnostics.push(diagnostic);
    }
    diagnostics
}
