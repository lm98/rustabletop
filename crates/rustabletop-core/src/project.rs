use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::component::{self, Kind, Orientation, Production, ProductionDoc, Shape, ShapeDoc};
use crate::diagnostics::{Diagnostic, Location, Subject};
use crate::grid::{self, Grid, GridDoc};
use crate::schema;
use crate::source::{self, Located, pointer_push};
use crate::template;
use crate::units::{Length, Size};

pub const MANIFEST_FILE: &str = "game.yaml";
const DEFAULT_BLEED: Length = Length::mm(3.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    A4,
    Letter,
}

impl Paper {
    pub fn size(self) -> Size {
        let (width, height) = match self {
            Paper::A4 => (210.0, 297.0),
            Paper::Letter => (215.9, 279.4),
        };
        Size {
            width: Length::mm(width),
            height: Length::mm(height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DuplexEdge {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Print {
    pub paper: Paper,
    pub margin: Length,
    pub duplex_edge: DuplexEdge,
}

impl Default for Print {
    fn default() -> Self {
        Self {
            paper: Paper::A4,
            margin: Length::mm(5.0),
            duplex_edge: DuplexEdge::Long,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDoc {
    game: String,
    #[serde(default)]
    icons: BTreeMap<String, String>,
    #[serde(default)]
    print: Print,
    fonts: Option<String>,
    components: Vec<ComponentDoc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentDoc {
    id: String,
    kind: Kind,
    shape: Option<Value>,
    size: Value,
    orientation: Option<Orientation>,
    radius: Option<Length>,
    bleed: Option<Length>,
    schema: Option<Value>,
    faces: BTreeMap<String, String>,
    data: Option<String>,
    fields: Option<Value>,
    print: Option<ProductionDoc>,
    grid: Option<GridDoc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    #[serde(skip)]
    pub root: PathBuf,
    pub game: String,
    pub icons: BTreeMap<String, String>,
    pub print: Print,
    pub fonts: Option<String>,
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Component {
    pub id: String,
    pub kind: Kind,
    pub shape: Shape,
    pub bleed: Length,
    pub faces: BTreeMap<String, String>,
    pub production: Production,
    pub grid: Option<Grid>,
    pub data: Option<String>,
    pub schema: Value,
    pub instances: Vec<Instance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Instance {
    pub id: String,
    pub qty: u64,
    pub fields: Map<String, Value>,
    #[serde(skip)]
    pub origin: Option<Location>,
    #[serde(skip)]
    pub field_origins: BTreeMap<String, Location>,
}

impl Instance {
    pub fn location(&self, field: Option<&str>) -> Option<Location> {
        field
            .and_then(|f| self.field_origins.get(f))
            .or(self.origin.as_ref())
            .cloned()
    }
}

impl Component {
    pub fn total_qty(&self) -> u64 {
        self.instances.iter().map(|i| i.qty).sum()
    }

    pub fn face_names(&self) -> Vec<&'static str> {
        self.kind
            .faces()
            .iter()
            .copied()
            .filter(|f| self.faces.contains_key(*f))
            .collect()
    }

    pub fn has_back(&self) -> bool {
        self.faces.contains_key("back")
    }
}

pub struct Checked {
    pub project: Option<Project>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Checked {
    pub fn ok(&self) -> bool {
        !self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

pub fn check(root: &Path) -> Checked {
    let inspected = inspect(root);
    let ok = inspected.ok();
    Checked {
        project: inspected.project.filter(|_| ok),
        diagnostics: inspected.diagnostics,
    }
}

pub fn inspect(root: &Path) -> Checked {
    let mut diagnostics = Vec::new();
    let project = load(root, &mut diagnostics);
    diagnostics.sort_by(|a, b| {
        let key = |d: &Diagnostic| {
            d.location
                .as_ref()
                .map(|l| (l.file.clone(), l.line, l.column))
        };
        key(a).cmp(&key(b))
    });
    Checked {
        project,
        diagnostics,
    }
}

fn load(root: &Path, diags: &mut Vec<Diagnostic>) -> Option<Project> {
    let manifest_path = root.join(MANIFEST_FILE);
    let text = match fs::read_to_string(&manifest_path) {
        Ok(t) => t,
        Err(e) => {
            diags.push(
                Diagnostic::error(
                    "manifest.missing",
                    format!("cannot read {}: {e}", manifest_path.display()),
                )
                .hint(format!(
                    "a project is a directory containing `{MANIFEST_FILE}`"
                )),
            );
            return None;
        }
    };
    let manifest = source::load_yaml(MANIFEST_FILE, &text)
        .map_err(|d| diags.push(d))
        .ok()?;
    let doc: ManifestDoc = match serde_path_to_error::deserialize(&manifest.value) {
        Ok(doc) => doc,
        Err(e) => {
            let pointer = serde_path_pointer(e.path());
            diags.push(
                Diagnostic::error("manifest.invalid", format!("{}: {}", e.path(), e.inner()))
                    .at(manifest.location(&pointer)),
            );
            return None;
        }
    };

    let icons = check_icons(root, &doc.icons, &manifest, diags);
    let fonts = check_fonts(root, doc.fonts.as_deref(), &manifest, diags);

    let shapes: Vec<(String, Option<Shape>)> = doc
        .components
        .iter()
        .map(|comp| {
            let shape = component::parse_shape(&ShapeDoc {
                kind: comp.kind,
                shape: comp.shape.as_ref(),
                size: &comp.size,
                orientation: comp.orientation,
                radius: comp.radius,
            })
            .ok();
            (comp.id.clone(), shape)
        })
        .collect();

    let mut components = Vec::new();
    let mut seen = HashMap::new();
    for (i, comp) in doc.components.into_iter().enumerate() {
        let base = format!("/components/{i}");
        let at = |suffix: &str| manifest.location(&format!("{base}{suffix}"));
        let subject = Subject {
            component: Some(comp.id.clone()),
            ..Subject::default()
        };

        if !is_slug(&comp.id) {
            diags.push(
                Diagnostic::error(
                    "manifest.id",
                    format!(
                        "component id `{}` must be lowercase words separated by dashes",
                        comp.id
                    ),
                )
                .at(at("/id")),
            );
        }
        if let Some(first) = seen.insert(comp.id.clone(), i) {
            diags.push(
                Diagnostic::error(
                    "manifest.duplicate-id",
                    format!(
                        "component id `{}` is already used by component #{first}",
                        comp.id
                    ),
                )
                .at(at("/id")),
            );
        }

        let shape = component::parse_shape(&ShapeDoc {
            kind: comp.kind,
            shape: comp.shape.as_ref(),
            size: &comp.size,
            orientation: comp.orientation,
            radius: comp.radius,
        })
        .map_err(|e| {
            let code = if e.key == "size" {
                "manifest.size"
            } else {
                "manifest.shape"
            };
            diags.push(
                Diagnostic::error(code, e.message)
                    .at(at(&format!("/{}", e.key)))
                    .about(subject.clone())
                    .hints(e.hints),
            )
        })
        .ok();

        let faces_ok = check_faces(comp.kind, &comp.faces, &at, &subject, diags);
        let grid = match (&comp.grid, comp.kind, shape) {
            (None, _, _) => Ok(None),
            (Some(_), kind, _) if kind != Kind::Board => {
                diags.push(
                    Diagnostic::error(
                        "manifest.grid",
                        format!("a {} cannot have a `grid`", kind.name()),
                    )
                    .at(at("/grid"))
                    .about(subject.clone())
                    .hint("grids are available on boards"),
                );
                Err(())
            }
            (Some(_), _, None) => Err(()),
            (Some(doc), _, Some(shape)) => grid::resolve(doc, shape.bbox(), &shapes)
                .map(Some)
                .map_err(|e| {
                    diags.push(
                        Diagnostic::error("manifest.grid", e.message)
                            .at(at(&format!("/grid/{}", e.key)))
                            .about(subject.clone())
                            .hints(e.hints),
                    )
                }),
        };
        let production = component::production(
            comp.kind,
            comp.faces.contains_key("back"),
            &comp.print.clone().unwrap_or_default(),
        )
        .map_err(|e| {
            let pointer = match e.key {
                "print" => "/print".to_owned(),
                key => format!("/print/{key}"),
            };
            diags.push(
                Diagnostic::error("manifest.print", e.message)
                    .at(at(&pointer))
                    .about(subject.clone())
                    .hints(e.hints),
            )
        })
        .ok();

        let user_schema = comp
            .schema
            .clone()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let uses_grid_cells = schema::formatted_paths(&user_schema)
            .iter()
            .any(|(_, f)| *f == schema::Format::GridCell);
        if uses_grid_cells && comp.grid.is_none() {
            diags.push(
                Diagnostic::error(
                    "manifest.schema",
                    "`format: grid-cell` needs a `grid` on the component",
                )
                .at(at("/schema"))
                .about(subject.clone())
                .hint("declare `grid: { type, cols, rows, cell }` on a board"),
            );
        }
        let issues = schema::lint(&user_schema);
        if !issues.is_empty() {
            for issue in issues {
                diags.push(
                    Diagnostic::error("manifest.schema", issue.message)
                        .at(manifest.location(&format!("{base}/schema{}", issue.pointer)))
                        .about(subject.clone())
                        .hints(issue.hints),
                );
            }
            continue;
        }

        let schema = match schema::effective_schema(&user_schema) {
            Ok(s) => s,
            Err(m) => {
                diags.push(
                    Diagnostic::error("manifest.schema", m)
                        .at(at("/schema"))
                        .about(subject),
                );
                continue;
            }
        };
        let validator = match jsonschema::validator_for(&schema) {
            Ok(v) => v,
            Err(e) => {
                let pointer = format!("{base}/schema{}", e.instance_path());
                diags.push(
                    Diagnostic::error("manifest.schema", format!("invalid JSON Schema: {e}"))
                        .at(manifest.location(&pointer))
                        .about(subject),
                );
                continue;
            }
        };

        for (face, path) in comp.faces.iter().filter(|_| faces_ok) {
            let face = face.as_str();
            let declared = at(&format!("/faces/{face}"));
            match read_inside(root, path) {
                None => diags.push(
                    Diagnostic::error(
                        "manifest.missing-file",
                        format!("face template `{path}` not found"),
                    )
                    .at(declared)
                    .about(subject.clone()),
                ),
                Some(text) if !template::defines(&text, face) => diags.push(
                    Diagnostic::error(
                        "manifest.face-function",
                        format!("face template `{path}` does not define `{face}`"),
                    )
                    .at(declared)
                    .about(subject.clone())
                    .hint(format!(
                        "the file must export `#let {face}(item, ctx) = {{ ... }}`"
                    )),
                ),
                Some(_) => {}
            }
        }

        let (data, records) = match (&comp.data, &comp.fields) {
            (Some(_), Some(_)) => {
                diags.push(
                    Diagnostic::error("manifest.data", "`data` and `fields` cannot be used together")
                        .at(at("/fields"))
                        .about(subject)
                        .hint("use `data` for a file of records, or `fields` for a single inline instance"),
                );
                continue;
            }
            (Some(rel), None) => {
                let Some(data) = load_data(root, rel, &schema, at("/data"), &subject, diags) else {
                    continue;
                };
                let records = data
                    .value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, r)| (format!("/{i}"), r.clone()))
                    .collect();
                (data, records)
            }
            (None, fields) => {
                let mut record = match fields {
                    None => Map::new(),
                    Some(Value::Object(map)) => map.clone(),
                    Some(_) => {
                        diags.push(
                            Diagnostic::error("manifest.data", "`fields` must be a mapping")
                                .at(at("/fields"))
                                .about(subject),
                        );
                        continue;
                    }
                };
                record
                    .entry("id")
                    .or_insert_with(|| Value::String(comp.id.clone()));
                let base = if fields.is_some() {
                    format!("{base}/fields")
                } else {
                    base.clone()
                };
                (manifest.clone(), vec![(base, Value::Object(record))])
            }
        };
        let context = RecordContext {
            root,
            component: &comp.id,
            schema: &schema,
            validator: &validator,
            icons: &icons,
            grid: grid.as_ref().ok().and_then(Option::as_ref),
        };
        let instances = validate_records(&data, &records, &context, diags);

        if let (Some(shape), Some(production), true, Ok(grid)) = (shape, production, faces_ok, grid)
        {
            components.push(Component {
                id: comp.id,
                kind: comp.kind,
                shape,
                bleed: comp.bleed.unwrap_or(DEFAULT_BLEED),
                faces: comp.faces,
                production,
                grid,
                data: comp.data,
                schema,
                instances,
            });
        }
    }

    Some(Project {
        root: root.to_path_buf(),
        game: doc.game,
        icons: doc.icons,
        print: doc.print,
        fonts,
        components,
    })
}

fn check_faces(
    kind: Kind,
    faces: &BTreeMap<String, String>,
    at: &dyn Fn(&str) -> Option<Location>,
    subject: &Subject,
    diags: &mut Vec<Diagnostic>,
) -> bool {
    let allowed: Vec<String> = kind.faces().iter().map(|f| (*f).to_owned()).collect();
    let before = diags.len();
    for name in faces.keys().filter(|n| !allowed.contains(n)) {
        diags.push(
            Diagnostic::error(
                "manifest.face",
                format!("a {} has no face `{name}`", kind.name()),
            )
            .at(at(&pointer_push("/faces", name)))
            .about(subject.clone())
            .hints(schema::suggest(name, &allowed))
            .hint(format!("faces: {}", allowed.join(", "))),
        );
    }
    if !faces.contains_key("front") {
        diags.push(
            Diagnostic::error("manifest.face", "missing required face `front`")
                .at(at("/faces"))
                .about(subject.clone())
                .hint(format!("faces: {}", allowed.join(", "))),
        );
    }
    diags.len() == before
}

fn read_inside(root: &Path, rel: &str) -> Option<String> {
    is_inside(rel)
        .then(|| fs::read_to_string(root.join(rel)).ok())
        .flatten()
}

fn is_inside(rel: &str) -> bool {
    let path = Path::new(rel);
    !rel.is_empty()
        && path.components().all(|c| {
            matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

fn check_icons(
    root: &Path,
    icons: &BTreeMap<String, String>,
    manifest: &Located,
    diags: &mut Vec<Diagnostic>,
) -> Vec<String> {
    for (name, path) in icons {
        let at = manifest.location(&pointer_push("/icons", name));
        if !is_icon_name(name) {
            diags.push(
                Diagnostic::error(
                    "manifest.icon",
                    format!("icon name `{name}` must be lowercase words separated by dashes, starting with a letter"),
                )
                .at(at.clone()),
            );
        }
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(extension.as_deref(), Some("svg" | "png")) {
            diags.push(
                Diagnostic::error(
                    "manifest.icon",
                    format!("icon `{path}` must be an SVG or PNG file"),
                )
                .at(at.clone()),
            );
        } else if !is_inside(path) || !root.join(path).is_file() {
            diags.push(
                Diagnostic::error(
                    "manifest.missing-file",
                    format!("icon file `{path}` not found"),
                )
                .at(at),
            );
        }
    }
    icons.keys().cloned().collect()
}

fn check_fonts(
    root: &Path,
    declared: Option<&str>,
    manifest: &Located,
    diags: &mut Vec<Diagnostic>,
) -> Option<String> {
    match declared {
        Some(dir) if is_inside(dir) && root.join(dir).is_dir() => Some(dir.to_owned()),
        Some(dir) => {
            diags.push(
                Diagnostic::error(
                    "manifest.missing-file",
                    format!("font directory `{dir}` not found in the project"),
                )
                .at(manifest.location("/fonts")),
            );
            None
        }
        None => root.join("fonts").is_dir().then(|| "fonts".to_owned()),
    }
}

pub(crate) fn is_icon_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase()) && is_slug(s)
}

fn load_data(
    root: &Path,
    rel: &str,
    schema: &Value,
    declared_at: Option<Location>,
    subject: &Subject,
    diags: &mut Vec<Diagnostic>,
) -> Option<Located> {
    let text = match fs::read_to_string(root.join(rel)) {
        Ok(t) => t,
        Err(e) => {
            diags.push(
                Diagnostic::error("manifest.missing-file", format!("data file `{rel}`: {e}"))
                    .at(declared_at)
                    .about(subject.clone()),
            );
            return None;
        }
    };
    let extension = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let loaded = match extension {
        "yaml" | "yml" => source::load_yaml(rel, &text),
        "csv" => source::load_csv(rel, &text, &|field| schema::field_types(schema, field)),
        other => Err(Diagnostic::error(
            "manifest.data-format",
            format!("unsupported data format `.{other}`"),
        )
        .at(declared_at.clone())
        .hint("use .yaml (preferred) or .csv")),
    };
    let data = loaded
        .map_err(|d| diags.push(d.about(subject.clone())))
        .ok()?;
    if !data.value.is_array() {
        diags.push(
            Diagnostic::error("data.shape", "data must be a list of records")
                .at(data.location(""))
                .about(subject.clone())
                .hint("in YAML, start each record with `- id: ...`"),
        );
        return None;
    }
    Some(data)
}

struct RecordContext<'a> {
    root: &'a Path,
    component: &'a str,
    schema: &'a Value,
    validator: &'a jsonschema::Validator,
    icons: &'a [String],
    grid: Option<&'a Grid>,
}

fn validate_records(
    data: &Located,
    records: &[(String, Value)],
    context: &RecordContext,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Instance> {
    let RecordContext {
        component,
        schema,
        validator,
        ..
    } = *context;
    let mut instances = Vec::new();
    let mut ids: HashMap<String, Option<Location>> = HashMap::new();
    for (i, (base, record)) in records.iter().enumerate() {
        let id = record.get("id").and_then(Value::as_str).map(str::to_owned);
        let subject = |field: Option<String>| Subject {
            component: Some(component.to_owned()),
            instance: id.clone(),
            field,
        };

        let Value::Object(fields) = record else {
            diags.push(
                Diagnostic::error("data.shape", format!("record #{} is not a mapping", i + 1))
                    .at(data.location(base))
                    .about(subject(None)),
            );
            continue;
        };

        let mut valid = true;
        for err in validator.iter_errors(record) {
            valid = false;
            for v in schema::describe(&err, schema) {
                diags.push(
                    Diagnostic::error(v.code, v.message)
                        .at(data.location(&format!("{base}{}", v.pointer)))
                        .about(subject(v.field))
                        .hints(v.hints),
                );
            }
        }

        if let Some(id) = &id {
            let here = data.location(&pointer_push(base, "id"));
            if let Some(first) = ids.insert(id.clone(), here.clone()) {
                let previous = first.map(|l| format!("first defined at {}:{}", l.file, l.line));
                diags.push(
                    Diagnostic::error("data.duplicate-id", format!("duplicate id `{id}`"))
                        .at(here)
                        .about(subject(Some("id".into())))
                        .hints(previous),
                );
                valid = false;
            }
        }

        if valid {
            valid = check_formats(fields, base, data, context, &subject, diags);
        }

        if valid {
            let field_origins = fields
                .keys()
                .filter_map(|k| Some((k.clone(), data.location(&pointer_push(base, k))?)))
                .collect();
            let mut fields = fields.clone();
            fields.remove("id");
            let qty = fields.remove("qty").and_then(|q| q.as_u64()).unwrap_or(1);
            instances.push(Instance {
                id: id.unwrap_or_default(),
                qty,
                fields,
                origin: data.location(base),
                field_origins,
            });
        }
    }
    instances
}

fn check_formats(
    fields: &Map<String, Value>,
    base: &str,
    data: &Located,
    context: &RecordContext,
    subject: &dyn Fn(Option<String>) -> Subject,
    diags: &mut Vec<Diagnostic>,
) -> bool {
    let before = diags.len();
    let record = Value::Object(fields.clone());
    for (path, format) in schema::formatted_paths(context.schema) {
        for (relative, value) in schema::select(&record, &path, String::new()) {
            let pointer = format!("{base}{relative}");
            let field = source::pointer_segments(&relative).join(".");
            let at = data.location(&pointer);
            let error = |code: &'static str, message: String| {
                Diagnostic::error(code, format!("`{field}`: {message}"))
                    .at(at.clone())
                    .about(subject(Some(field.clone())))
            };
            match (format, value) {
                (schema::Format::RichText, Value::String(text)) => {
                    for token in template::icon_tokens(text) {
                        if context.icons.iter().any(|i| i == token) {
                            continue;
                        }
                        let hints = schema::suggest(token, context.icons).into_iter().chain([
                            if context.icons.is_empty() {
                                "no icons are declared: add them under `icons:` in game.yaml"
                                    .to_owned()
                            } else {
                                format!("declared icons: {}", context.icons.join(", "))
                            },
                        ]);
                        diags.push(
                            error("data.unknown-icon", format!("unknown icon `:{token}:`"))
                                .hints(hints),
                        );
                    }
                }
                (schema::Format::Asset, Value::String(text)) => {
                    if !is_inside(text) || !context.root.join(text).is_file() {
                        diags.push(
                            error(
                                "data.missing-asset",
                                format!("asset `{text}` not found in the project"),
                            )
                            .hints(sibling_suggestion(context.root, text)),
                        );
                    }
                }
                (schema::Format::GridCell, value) => {
                    let Some(grid) = context.grid else { continue };
                    let cell = value
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .and_then(|a| Some((a[0].as_i64()?, a[1].as_i64()?)));
                    let problem = match cell {
                        None => Some(format!("{value} is not a `[col, row]` pair of integers")),
                        Some((col, row)) if !grid.contains(col, row) => {
                            Some(format!("[{col}, {row}] is outside the grid"))
                        }
                        Some(_) => None,
                    };
                    if let Some(message) = problem {
                        diags.push(error("data.grid-cell", message).hint(grid.describe_bounds()));
                    }
                }
                _ => {}
            }
        }
    }
    diags.len() == before
}

fn sibling_suggestion(root: &Path, rel: &str) -> Option<String> {
    let path = Path::new(rel);
    let name = path.file_name()?.to_str()?;
    let dir = root.join(path.parent()?);
    let siblings: Vec<String> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    schema::suggest(name, &siblings)
}

fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn serde_path_pointer(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    path.iter().fold(String::new(), |acc, seg| match seg {
        Segment::Seq { index } => pointer_push(&acc, &index.to_string()),
        Segment::Map { key } => pointer_push(&acc, key),
        Segment::Enum { variant } => pointer_push(&acc, variant),
        Segment::Unknown => acc,
    })
}

#[cfg(test)]
pub(crate) fn manifest_keys() -> Vec<String> {
    use serde::de::DeserializeOwned;

    fn fields<T: DeserializeOwned>() -> Vec<String> {
        let probe = serde_json::json!({ "rtt-probe": 0 });
        let message = serde_json::from_value::<T>(probe)
            .err()
            .expect("the probe key is rejected")
            .to_string();
        let expected = message
            .split_once("expected")
            .map(|(_, rest)| rest)
            .unwrap_or_default();
        expected
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    }

    [
        fields::<ManifestDoc>(),
        fields::<Print>(),
        fields::<ComponentDoc>(),
        fields::<ProductionDoc>(),
        fields::<GridDoc>(),
        vec!["width".into(), "height".into()],
    ]
    .concat()
}
