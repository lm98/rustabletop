use std::collections::HashMap;

use saphyr::{LoadableYamlNode, MarkedYaml, Scalar, YamlData};
use serde_json::{Map, Number, Value};

use crate::diagnostics::{Diagnostic, Location};

/// A JSON value loaded from a source file, with the source position of every node keyed by JSON pointer.
#[derive(Debug, Clone)]
pub struct Located {
    pub value: Value,
    pub file: String,
    positions: HashMap<String, (usize, Option<usize>)>,
}

impl Located {
    pub fn location(&self, pointer: &str) -> Option<Location> {
        let mut pointer = pointer;
        loop {
            if let Some(&(line, column)) = self.positions.get(pointer) {
                return Some(Location::new(self.file.clone(), line, column));
            }
            {
                let cut = pointer.rfind('/')?;
                pointer = &pointer[..cut]
            }
        }
    }
}

pub fn pointer_push(base: &str, segment: &str) -> String {
    format!("{base}/{}", segment.replace('~', "~0").replace('/', "~1"))
}

pub fn pointer_segments(pointer: &str) -> Vec<String> {
    pointer
        .split('/')
        .skip(1)
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect()
}

pub fn load_yaml(file: &str, text: &str) -> Result<Located, Diagnostic> {
    let docs = MarkedYaml::load_from_str(text).map_err(|e| {
        Diagnostic::error("yaml.syntax", e.info().to_string()).at(Some(Location::new(
            file,
            e.marker().line(),
            Some(e.marker().col() + 1),
        )))
    })?;
    let mut located = Located {
        value: Value::Null,
        file: file.to_owned(),
        positions: HashMap::new(),
    };
    match docs.as_slice() {
        [] => Ok(located),
        [doc] => {
            located.value = convert(doc, String::new(), &mut located.positions, file)?;
            Ok(located)
        }
        _ => Err(
            Diagnostic::error("yaml.multiple-documents", "expected a single YAML document")
                .at(Some(Location::new(file, 1, None))),
        ),
    }
}

fn convert(
    node: &MarkedYaml,
    pointer: String,
    positions: &mut HashMap<String, (usize, Option<usize>)>,
    file: &str,
) -> Result<Value, Diagnostic> {
    let start = node.span.start;
    positions
        .entry(pointer.clone())
        .or_insert((start.line(), Some(start.col() + 1)));
    let here = || Some(Location::new(file, start.line(), Some(start.col() + 1)));

    Ok(match &node.data {
        YamlData::Value(scalar) => scalar_to_json(scalar),
        YamlData::Representation(raw, _, _) => Value::String(raw.to_string()),
        YamlData::Tagged(_, inner) => convert(inner, pointer, positions, file)?,
        YamlData::Sequence(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    convert(
                        item,
                        pointer_push(&pointer, &i.to_string()),
                        positions,
                        file,
                    )
                })
                .collect::<Result<_, _>>()?,
        ),
        YamlData::Mapping(entries) => {
            let mut map = Map::new();
            for (key, value) in entries {
                let key_text = match &key.data {
                    YamlData::Value(Scalar::String(s)) => s.to_string(),
                    YamlData::Value(other) => scalar_to_json(other).to_string(),
                    _ => {
                        return Err(Diagnostic::error(
                            "yaml.unsupported",
                            "mapping keys must be scalars",
                        )
                        .at(here()));
                    }
                };
                let child = pointer_push(&pointer, &key_text);
                let k = key.span.start;
                positions.insert(child.clone(), (k.line(), Some(k.col() + 1)));
                map.insert(key_text, convert(value, child, positions, file)?);
            }
            Value::Object(map)
        }
        YamlData::Alias(_) => {
            return Err(
                Diagnostic::error("yaml.unsupported", "YAML aliases are not supported").at(here()),
            );
        }
        _ => return Err(Diagnostic::error("yaml.unsupported", "unsupported YAML node").at(here())),
    })
}

fn scalar_to_json(scalar: &Scalar) -> Value {
    match scalar {
        Scalar::Null => Value::Null,
        Scalar::Boolean(b) => Value::Bool(*b),
        Scalar::Integer(i) => Value::from(*i),
        Scalar::FloatingPoint(f) => {
            Number::from_f64(f.into_inner()).map_or(Value::Null, Value::Number)
        }
        Scalar::String(s) => Value::String(s.to_string()),
    }
}

pub fn load_csv(
    file: &str,
    text: &str,
    field_types: &dyn Fn(&str) -> Vec<String>,
) -> Result<Located, Diagnostic> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| csv_error(file, &e))?
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let mut positions = HashMap::new();
    let mut rows = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let record = record.map_err(|e| csv_error(file, &e))?;
        let line = record.position().map_or(0, |p| p.line() as usize);
        let pointer = format!("/{i}");
        positions.insert(pointer.clone(), (line, None));

        let mut row = Map::new();
        for (header, cell) in headers.iter().zip(record.iter()) {
            if cell.is_empty() {
                continue;
            }
            positions.insert(pointer_push(&pointer, header), (line, None));
            row.insert(header.clone(), coerce(cell, &field_types(header)));
        }
        rows.push(Value::Object(row));
    }
    Ok(Located {
        value: Value::Array(rows),
        file: file.to_owned(),
        positions,
    })
}

fn coerce(cell: &str, types: &[String]) -> Value {
    let wants = |t: &str| types.iter().any(|x| x == t);
    if wants("integer")
        && let Ok(i) = cell.parse::<i64>()
    {
        return Value::from(i);
    }
    if wants("number")
        && let Some(n) = cell.parse::<f64>().ok().and_then(Number::from_f64)
    {
        return Value::Number(n);
    }
    if wants("boolean") {
        match cell.to_ascii_lowercase().as_str() {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            _ => {}
        }
    }
    Value::String(cell.to_owned())
}

fn csv_error(file: &str, error: &csv::Error) -> Diagnostic {
    let line = error.position().map_or(1, |p| p.line() as usize);
    let diagnostic = Diagnostic::error("csv.syntax", error.to_string())
        .at(Some(Location::new(file, line, None)));
    match error.kind() {
        csv::ErrorKind::UnequalLengths { .. } => {
            diagnostic.hint("a field probably contains a comma: wrap it in double quotes, e.g. \"text, with comma\"")
        }
        _ => diagnostic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_positions_follow_keys() {
        let text = "- id: a\n  cost: 1\n- id: b\n  cost: two\n";
        let doc = load_yaml("d.yaml", text).unwrap();
        assert_eq!(doc.location("/1/cost").unwrap().line, 4);
        assert_eq!(doc.location("/1").unwrap().line, 3);
        assert_eq!(doc.location("/1/cost/deep").unwrap().line, 4);
    }

    #[test]
    fn csv_coerces_by_schema_type() {
        let text = "id,cost,name\na,3,Foo\nb,three,\n";
        let types = |f: &str| {
            if f == "cost" {
                vec!["integer".to_owned()]
            } else {
                vec![]
            }
        };
        let doc = load_csv("d.csv", text, &types).unwrap();
        assert_eq!(doc.value[0]["cost"], Value::from(3));
        assert_eq!(doc.value[1]["cost"], Value::from("three"));
        assert!(doc.value[1].get("name").is_none());
        assert_eq!(doc.location("/1/cost").unwrap().line, 3);
    }

    #[test]
    fn csv_unquoted_comma_is_reported_with_hint() {
        let text = "id,text\na,hello, world\n";
        let err = load_csv("d.csv", text, &|_| vec![]).unwrap_err();
        assert_eq!(err.location.unwrap().line, 2);
        assert!(err.hints[0].contains("double quotes"));
    }
}
