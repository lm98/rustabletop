use jsonschema::ValidationError;
use jsonschema::error::{TypeKind, ValidationErrorKind};
use serde_json::{Map, Value, json};

use crate::source::pointer_segments;

pub const ID_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";
pub const RESERVED_FIELDS: [&str; 2] = ["id", "qty"];

pub fn effective_schema(user: &Value) -> Result<Value, String> {
    let Value::Object(user) = user else {
        return Err("schema must be a JSON Schema object".into());
    };
    let mut schema = user.clone();
    match schema.get("type") {
        None => {
            schema.insert("type".into(), "object".into());
        }
        Some(Value::String(t)) if t == "object" => {}
        Some(_) => return Err("schema `type` must be \"object\" (a record)".into()),
    }

    let properties = schema
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("schema `properties` must be an object")?;
    if let Some(reserved) = RESERVED_FIELDS
        .iter()
        .find(|f| properties.contains_key(**f))
    {
        return Err(format!(
            "`{reserved}` is reserved and defined by rustabletop; remove it from the schema"
        ));
    }
    properties.insert(
        "id".into(),
        json!({ "type": "string", "pattern": ID_PATTERN }),
    );
    properties.insert("qty".into(), json!({ "type": "integer", "minimum": 0 }));

    let required = schema
        .entry("required")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("schema `required` must be an array")?;
    if !required.iter().any(|r| r == "id") {
        required.insert(0, "id".into());
    }

    schema
        .entry("additionalProperties")
        .or_insert(Value::Bool(false));
    Ok(Value::Object(schema))
}

const KEYWORDS: &[&str] = &[
    "type",
    "enum",
    "const",
    "default",
    "title",
    "description",
    "examples",
    "format",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minLength",
    "maxLength",
    "pattern",
    "items",
    "minItems",
    "maxItems",
    "uniqueItems",
    "properties",
    "required",
    "additionalProperties",
    "anyOf",
    "oneOf",
    "allOf",
    "not",
];
pub const FORMATS: &[&str] = &["rich-text", "asset"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    RichText,
    Asset,
}

impl Format {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "rich-text" => Some(Self::RichText),
            "asset" => Some(Self::Asset),
            _ => None,
        }
    }
}

pub fn formatted_fields(schema: &Value) -> Vec<(String, Format)> {
    let format_of = |node: &Value| {
        node.get("format")
            .and_then(Value::as_str)
            .and_then(Format::parse)
    };
    schema
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, prop)| {
            let format = format_of(prop).or_else(|| prop.get("items").and_then(format_of))?;
            Some((name.clone(), format))
        })
        .collect()
}

const TYPES: &[&str] = &[
    "string", "integer", "number", "boolean", "array", "object", "null",
];

pub struct SchemaIssue {
    pub pointer: String,
    pub message: String,
    pub hints: Vec<String>,
}

pub fn lint(schema: &Value) -> Vec<SchemaIssue> {
    let mut issues = Vec::new();
    lint_node(schema, String::new(), &mut issues);
    issues
}

fn lint_node(node: &Value, pointer: String, issues: &mut Vec<SchemaIssue>) {
    let Value::Object(map) = node else {
        if !node.is_boolean() {
            issues.push(SchemaIssue {
                pointer,
                message: "a schema must be a mapping".into(),
                hints: Vec::new(),
            });
        }
        return;
    };
    let keywords: Vec<String> = KEYWORDS.iter().map(|k| (*k).to_owned()).collect();
    let types: Vec<String> = TYPES.iter().map(|t| (*t).to_owned()).collect();

    for (key, value) in map {
        let here = crate::source::pointer_push(&pointer, key);
        if !KEYWORDS.contains(&key.as_str()) {
            issues.push(SchemaIssue {
                pointer: here,
                message: format!("unsupported schema keyword `{key}`"),
                hints: suggest(key, &keywords).into_iter().collect(),
            });
            continue;
        }
        match key.as_str() {
            "type" => {
                let names: Vec<&Value> = match value {
                    Value::Array(a) => a.iter().collect(),
                    v => vec![v],
                };
                for name in names {
                    match name.as_str() {
                        Some(t) if TYPES.contains(&t) => {}
                        Some(t) => issues.push(SchemaIssue {
                            pointer: here.clone(),
                            message: format!("unknown type `{t}`"),
                            hints: suggest(t, &types)
                                .into_iter()
                                .chain([format!("types: {}", TYPES.join(", "))])
                                .collect(),
                        }),
                        None => issues.push(SchemaIssue {
                            pointer: here.clone(),
                            message: "`type` must be a string or a list of strings".into(),
                            hints: Vec::new(),
                        }),
                    }
                }
            }
            "format" => match value.as_str() {
                Some(f) if FORMATS.contains(&f) => {}
                other => {
                    let formats: Vec<String> = FORMATS.iter().map(|f| (*f).to_owned()).collect();
                    issues.push(SchemaIssue {
                        pointer: here,
                        message: match other {
                            Some(f) => format!("unknown format `{f}`"),
                            None => "`format` must be a string".into(),
                        },
                        hints: other
                            .and_then(|f| suggest(f, &formats))
                            .into_iter()
                            .chain([format!("formats: {}", FORMATS.join(", "))])
                            .collect(),
                    });
                }
            },
            "properties" => {
                if let Value::Object(props) = value {
                    for (name, sub) in props {
                        lint_node(sub, crate::source::pointer_push(&here, name), issues);
                    }
                }
            }
            "items" | "not" => lint_node(value, here, issues),
            "additionalProperties" if value.is_object() => lint_node(value, here, issues),
            "anyOf" | "oneOf" | "allOf" => {
                if let Value::Array(subs) = value {
                    for (i, sub) in subs.iter().enumerate() {
                        lint_node(
                            sub,
                            crate::source::pointer_push(&here, &i.to_string()),
                            issues,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn field_types(schema: &Value, field: &str) -> Vec<String> {
    match schema.pointer(&format!("/properties/{field}/type")) {
        Some(Value::String(t)) => vec![t.clone()],
        Some(Value::Array(ts)) => ts
            .iter()
            .filter_map(|t| t.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn property_names(schema: &Value) -> Vec<String> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default()
}

pub struct Violation {
    pub code: &'static str,
    pub message: String,
    pub field: Option<String>,
    pub pointer: String,
    pub hints: Vec<String>,
}

pub fn describe(err: &ValidationError, schema: &Value) -> Vec<Violation> {
    let pointer = err.instance_path().to_string();
    let path = pointer_segments(&pointer);
    let field = (!path.is_empty()).then(|| path.join("."));
    let shown = field.clone().unwrap_or_else(|| "record".into());
    let found = err.instance();

    let violation = |code, message: String, hints: Vec<String>| Violation {
        code,
        message,
        field: field.clone(),
        pointer: pointer.clone(),
        hints,
    };

    match err.kind() {
        ValidationErrorKind::Enum { options } => {
            let allowed: Vec<String> = options
                .as_array()
                .map(|a| a.iter().map(display_plain).collect())
                .unwrap_or_default();
            let mut hints = vec![format!("allowed values: {}", allowed.join(", "))];
            if let Some(s) = found.as_str() {
                hints.extend(suggest(s, &allowed));
            }
            vec![violation(
                "data.enum",
                format!("`{shown}`: {found} is not an allowed value"),
                hints,
            )]
        }
        ValidationErrorKind::Type { kind } => {
            let expected = match kind {
                TypeKind::Single(t) => t.to_string(),
                TypeKind::Multiple(_) => field_types(schema, &shown).join(" or "),
            };
            vec![violation(
                "data.type",
                format!(
                    "`{shown}`: expected {expected}, found {} {found}",
                    json_type(found)
                ),
                Vec::new(),
            )]
        }
        ValidationErrorKind::Required { property } => {
            let name = display_plain(property);
            vec![Violation {
                code: "data.required",
                message: format!("missing required field `{name}`"),
                field: Some(name),
                pointer: pointer.clone(),
                hints: Vec::new(),
            }]
        }
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            let known = property_names(schema);
            unexpected
                .iter()
                .map(|name| Violation {
                    code: "data.unknown-field",
                    message: format!("unknown field `{name}`"),
                    field: Some(name.clone()),
                    pointer: crate::source::pointer_push(&pointer, name),
                    hints: suggest(name, &known)
                        .into_iter()
                        .chain([format!("declared fields: {}", known.join(", "))])
                        .collect(),
                })
                .collect()
        }
        ValidationErrorKind::Pattern { .. } if shown == "id" => vec![violation(
            "data.id",
            format!("`id` {found} must be lowercase words separated by dashes"),
            vec!["example: `ember-adept`".into()],
        )],
        _ => vec![violation(
            "data.invalid",
            format!("`{shown}`: {err}"),
            Vec::new(),
        )],
    }
}

fn display_plain(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn suggest(input: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|c| {
            (
                strsim::damerau_levenshtein(&input.to_lowercase(), &c.to_lowercase()),
                c,
            )
        })
        .filter(|(d, c)| *d > 0 && *d <= 2.max(c.len() / 4))
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| format!("did you mean `{c}`?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_reserved_fields_and_strictness() {
        let schema =
            effective_schema(&json!({ "properties": { "name": { "type": "string" } } })).unwrap();
        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["required"], json!(["id"]));
        assert!(schema["properties"]["qty"].is_object());
    }

    #[test]
    fn rejects_reserved_fields() {
        assert!(effective_schema(&json!({ "properties": { "qty": {} } })).is_err());
    }

    #[test]
    fn lint_catches_keyword_and_type_typos() {
        let issues = lint(&json!({
            "properties": {
                "suit": { "enumm": ["a"] },
                "cost": { "type": "integr" },
                "tags": { "type": "array", "items": { "type": "strng" } }
            }
        }));
        let pointers: Vec<&str> = issues.iter().map(|i| i.pointer.as_str()).collect();
        assert_eq!(
            pointers,
            [
                "/properties/cost/type",
                "/properties/suit/enumm",
                "/properties/tags/items/type"
            ]
        );
        assert_eq!(issues[1].hints, ["did you mean `enum`?"]);
    }

    #[test]
    fn lint_rejects_unknown_formats() {
        let issues = lint(&json!({ "properties": { "text": { "format": "rich-txt" } } }));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].hints[0], "did you mean `rich-text`?");
        assert!(lint(&json!({ "properties": { "art": { "format": "asset" } } })).is_empty());
    }

    #[test]
    fn finds_formatted_fields() {
        let schema = json!({ "properties": {
            "text": { "type": "string", "format": "rich-text" },
            "art": { "type": "array", "items": { "format": "asset" } },
            "name": { "type": "string" }
        }});
        let mut fields = formatted_fields(&schema);
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            fields,
            [
                ("art".to_owned(), Format::Asset),
                ("text".to_owned(), Format::RichText)
            ]
        );
    }

    #[test]
    fn suggests_close_matches() {
        let suits: Vec<String> = ["fuoco", "acqua", "terra", "aria"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            suggest("aira", &suits).as_deref(),
            Some("did you mean `aria`?")
        );
        assert_eq!(suggest("legno", &suits), None);
    }
}
