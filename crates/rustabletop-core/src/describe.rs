use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::component::{BoardMode, Shape};
use crate::diagnostics::Diagnostic;
use crate::grid::{Grid, GridType};
use crate::project::{self, Component, Project};
use crate::schema::FORMATS;
use crate::units::Length;

#[derive(Debug, Serialize)]
pub struct Description {
    pub ok: bool,
    pub game: Option<String>,
    pub print: Option<Value>,
    pub icons: Vec<String>,
    pub components: Vec<Value>,
    pub formats: Vec<&'static str>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn describe(root: &Path) -> Description {
    let inspected = project::inspect(root);
    let ok = inspected.ok();
    let project = inspected.project.as_ref();
    Description {
        ok,
        game: project.map(|p| p.game.clone()),
        print: project.map(print),
        icons: project
            .map(|p| p.icons.keys().cloned().collect())
            .unwrap_or_default(),
        components: project
            .map(|p| p.components.iter().map(component).collect())
            .unwrap_or_default(),
        formats: FORMATS.to_vec(),
        diagnostics: inspected.diagnostics,
    }
}

fn length(l: Length) -> Value {
    Value::String(l.to_string())
}

fn print(project: &Project) -> Value {
    let p = &project.print;
    json!({
        "paper": name(p.paper),
        "margin": length(p.margin),
        "duplex-edge": name(p.duplex_edge),
    })
}

fn shape(shape: &Shape) -> Value {
    match *shape {
        Shape::Rect { size, radius } => json!({
            "type": "rect", "width": length(size.width), "height": length(size.height),
            "radius": length(radius),
        }),
        Shape::Circle { diameter } => json!({ "type": "circle", "diameter": length(diameter) }),
        Shape::Square { side, radius } => {
            json!({ "type": "square", "side": length(side), "radius": length(radius) })
        }
        Shape::Hex {
            across_flats,
            orientation,
        } => json!({
            "type": "hex", "orientation": name(orientation), "across-flats": length(across_flats),
        }),
    }
}

fn production(component: &Component) -> Value {
    let p = &component.production;
    let mut out = Map::new();
    for option in component.kind.print_options() {
        let value = match *option {
            "sides" => name(p.sides),
            "crop-marks" => p.crop_marks.into(),
            "cut-lines" => p.cut_lines.into(),
            "gap" => length(p.gap),
            "mode" => name(p.mode.unwrap_or(BoardMode::Tiles)),
            "overlap" => length(p.overlap),
            _ => continue,
        };
        out.insert((*option).into(), value);
    }
    Value::Object(out)
}

fn name(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn grid(grid: &Grid) -> Value {
    json!({
        "type": name(grid.kind),
        "orientation": (grid.kind == GridType::Hex).then(|| name(grid.orientation)),
        "cols": grid.cols,
        "rows": grid.rows,
        "cell": length(Length::mm(grid.cell)),
        "origin": [length(Length::mm(grid.origin.0)), length(Length::mm(grid.origin.1))],
    })
}

fn component(c: &Component) -> Value {
    json!({
        "id": c.id,
        "kind": c.kind.name(),
        "shape": shape(&c.shape),
        "bleed": length(c.bleed),
        "faces": c.faces,
        "print": production(c),
        "grid": c.grid.as_ref().map(grid),
        "data": c.data,
        "instances": c.instances.len(),
        "copies": c.total_qty(),
        "schema": c.schema,
    })
}

impl Description {
    pub fn summary(&self) -> String {
        let mut out = String::new();
        let Some(game) = &self.game else {
            return out;
        };
        out.push_str(&format!("game: {game}\n"));
        if let Some(print) = &self.print {
            out.push_str(&format!(
                "print: {} paper, {} margin, duplex on the {} edge\n",
                print["paper"].as_str().unwrap_or_default(),
                print["margin"].as_str().unwrap_or_default(),
                print["duplex-edge"].as_str().unwrap_or_default()
            ));
        }
        if !self.icons.is_empty() {
            out.push_str(&format!("icons: {}\n", self.icons.join(", ")));
        }
        for c in &self.components {
            let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
            let shape = &c["shape"];
            let geometry = match text(&shape["type"]).as_str() {
                "rect" => format!("{} × {}", text(&shape["width"]), text(&shape["height"])),
                "circle" => format!("circle ⌀ {}", text(&shape["diameter"])),
                "square" => format!("square {}", text(&shape["side"])),
                _ => format!(
                    "hex {} {}",
                    text(&shape["orientation"]),
                    text(&shape["across-flats"])
                ),
            };
            out.push_str(&format!(
                "\n{} ({}, {geometry}, bleed {})\n",
                text(&c["id"]),
                text(&c["kind"]),
                text(&c["bleed"])
            ));
            let faces: Vec<String> = c["faces"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, v)| format!("{k} → {}", text(v)))
                .collect();
            out.push_str(&format!("  faces: {}\n", faces.join(", ")));
            let print: Vec<String> = c["print"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, v)| format!("{k}: {}", v.as_str().map_or(v.to_string(), str::to_owned)))
                .collect();
            out.push_str(&format!("  print: {}\n", print.join(", ")));
            if let Some(g) = c["grid"].as_object() {
                out.push_str(&format!(
                    "  grid: {} {} × {}, cell {}\n",
                    g["type"].as_str().unwrap_or_default(),
                    g["cols"],
                    g["rows"],
                    g["cell"].as_str().unwrap_or_default()
                ));
            }
            let source = c["data"].as_str().unwrap_or("inline `fields`");
            out.push_str(&format!(
                "  data: {source} ({} instances, {} copies)\n",
                c["instances"], c["copies"]
            ));
            out.push_str(&format!("  fields: {}\n", fields(&c["schema"])));
        }
        out
    }
}

fn fields(schema: &Value) -> String {
    let required: Vec<&str> = schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    schema["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, prop)| {
            let kind = if let Some(values) = prop["enum"].as_array() {
                let values: Vec<String> = values
                    .iter()
                    .map(|v| v.as_str().map_or(v.to_string(), str::to_owned))
                    .collect();
                values.join("|")
            } else if let Some(format) = prop["format"].as_str() {
                format.to_owned()
            } else {
                prop["type"].as_str().unwrap_or("any").to_owned()
            };
            let mark = if required.contains(&name.as_str()) {
                ""
            } else {
                "?"
            };
            format!("{name}{mark}: {kind}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
