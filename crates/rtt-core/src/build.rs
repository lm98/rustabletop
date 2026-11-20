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
use crate::impose::{self, Grid, Sheet, Side};
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
        let instance = &component.instances[preview.card];
        let dir = options.out.join(&component.id);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-{}.png", instance.id, side_name(preview.face)));
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

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Front => "front",
        Side::Back => "back",
    }
}

#[derive(Debug, Serialize)]
struct Preview {
    component: usize,
    card: usize,
    face: Side,
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
            let Some(grid) = Grid::new(&project.print, component.size, bleed) else {
                errors.push(
                    Diagnostic::error(
                        "print.too-large",
                        format!(
                            "a {} × {} card with {} bleed does not fit on {:?} paper with {} margins",
                            component.size.width,
                            component.size.height,
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
            let has_back = component.faces.back.is_some();
            sheets.extend(impose::sheets(
                index,
                &copies,
                &grid,
                bleed,
                &project.print,
                has_back,
            ));
            for card in 0..component.instances.len() {
                previews.push(Preview {
                    component: index,
                    card,
                    face: Side::Front,
                });
                if has_back {
                    previews.push(Preview {
                        component: index,
                        card,
                        face: Side::Back,
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
    let cards: Vec<Value> = component
        .instances
        .iter()
        .map(|instance| {
            let mut card = Map::new();
            card.insert("id".into(), instance.id.clone().into());
            card.insert("qty".into(), instance.qty.into());
            for name in &properties {
                if name != "id" && name != "qty" {
                    let value = instance.fields.get(name).cloned().unwrap_or(Value::Null);
                    card.insert(name.clone(), value);
                }
            }
            Value::Object(card)
        })
        .collect();
    let assets: Vec<String> = schema::formatted_fields(&component.schema)
        .into_iter()
        .filter(|(_, format)| *format == Format::Asset)
        .map(|(name, _)| name)
        .collect();
    json!({
        "id": component.id,
        "trim": [component.size.width.as_mm(), component.size.height.as_mm()],
        "bleed": component.bleed.as_mm(),
        "assets": assets,
        "cards": cards,
    })
}

fn main_source(project: &Project) -> String {
    let mut src = String::new();
    let mut faces = String::new();
    for (i, component) in project.components.iter().enumerate() {
        let mut entry = String::new();
        for (face, path) in [
            ("front", Some(&component.faces.front)),
            ("back", component.faces.back.as_ref()),
        ] {
            if let Some(path) = path {
                let alias = format!("rtt-face-{i}-{face}");
                let _ = writeln!(src, "#import {:?}: {face} as {alias}", format!("/{path}"));
                let _ = write!(entry, "{face}: {alias}, ");
            }
        }
        let _ = write!(faces, "({entry}), ");
    }
    let _ = writeln!(src, "#let rtt-faces = ({faces})");
    src.push_str(&MAIN_BODY.replace("{DATA}", DATA));
    src
}

const MAIN_BODY: &str = r#"#let rtt-data = json("{DATA}")
#let rtt-mm(v) = v * 1mm
#let rtt-path(v) = if v == none { none } else if type(v) == array { v.map(rtt-path) } else { path("/" + v) }
#let rtt-components = rtt-data.components.map(c => {
  let cards = c.cards.map(card => {
    for field in c.assets { card.insert(field, rtt-path(card.at(field))) }
    card
  })
  (..c, cards: cards)
})

#let rtt-face(ci, k, face) = {
  let c = rtt-components.at(ci)
  let card = c.cards.at(k)
  let ctx = (
    trim: (width: rtt-mm(c.trim.at(0)), height: rtt-mm(c.trim.at(1))),
    bleed: rtt-mm(c.bleed),
    component: c.id,
    face: face,
  )
  box(width: ctx.trim.width + 2 * ctx.bleed, height: ctx.trim.height + 2 * ctx.bleed, clip: true, {
    place([#metadata((component: c.id, card: card.id, face: face)) <rtt-face>])
    rtt-faces.at(ci).at(face)(card, ctx)
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
    for cell in sheet.cells {
      let face = rtt-face(sheet.component, cell.card, sheet.face)
      if sheet.rotate { face = rotate(180deg, face) }
      place(top + left, dx: rtt-mm(cell.x - cell.l), dy: rtt-mm(cell.y - cell.t),
        box(width: rtt-mm(w + cell.l + cell.r), height: rtt-mm(h + cell.t + cell.b), clip: true,
          place(top + left, dx: rtt-mm(cell.l - b), dy: rtt-mm(cell.t - b), face)))
    }
  })
}

#for p in rtt-data.previews {
  let c = rtt-components.at(p.component)
  page(width: rtt-mm(c.trim.at(0) + 2 * c.bleed), height: rtt-mm(c.trim.at(1) + 2 * c.bleed), margin: 0pt,
    rtt-face(p.component, p.card, p.face))
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
        let (Some(component), Some(card), Some(side)) = (
            text(face, "component"),
            text(face, "card"),
            text(face, "face"),
        ) else {
            continue;
        };
        let field = text(&value, "field");
        let size = value
            .get("size")
            .and_then(Value::as_f64)
            .unwrap_or_default();
        if !seen.insert((component.clone(), card.clone(), side.clone(), field.clone())) {
            continue;
        }
        let instance = project
            .components
            .iter()
            .find(|c| c.id == component)
            .and_then(|c| c.instances.iter().find(|i| i.id == card));
        let what = match &field {
            Some(f) => format!("`{f}`"),
            None => format!("a `fit` block on the {side}"),
        };
        let mut diagnostic = Diagnostic::warning(
            "render.overflow",
            format!("{what} of {component}/{card} does not fit at {size}pt"),
        )
        .at(instance.and_then(|i| i.location(field.as_deref())))
        .about(Subject {
            component: Some(component),
            instance: Some(card),
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
