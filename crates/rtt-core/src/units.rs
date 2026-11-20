use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Length {
    mm: f64,
}

impl Length {
    pub const fn mm(mm: f64) -> Self {
        Self { mm }
    }

    pub fn as_mm(self) -> f64 {
        self.mm
    }
}

impl TryFrom<String> for Length {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl std::str::FromStr for Length {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let split = s
            .find(|c: char| c.is_ascii_alphabetic())
            .ok_or_else(|| format!("length `{s}` has no unit (use mm, cm, in or pt)"))?;
        let (number, unit) = s.split_at(split);
        let value: f64 = number
            .trim()
            .parse()
            .map_err(|_| format!("length `{s}` does not start with a number"))?;
        let factor = match unit {
            "mm" => 1.0,
            "cm" => 10.0,
            "in" => 25.4,
            "pt" => 25.4 / 72.0,
            other => {
                return Err(format!(
                    "unknown unit `{other}` in `{s}` (use mm, cm, in or pt)"
                ));
            }
        };
        if !value.is_finite() || value < 0.0 {
            return Err(format!("length `{s}` must be a non-negative number"));
        }
        Ok(Self::mm(value * factor))
    }
}

impl std::fmt::Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mm", self.mm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    pub width: Length,
    pub height: Length,
}

pub const SIZE_PRESETS: &[(&str, f64, f64)] = &[
    ("poker", 63.0, 88.0),
    ("tarot", 70.0, 120.0),
    ("mini-euro", 44.0, 68.0),
    ("mini-usa", 41.0, 63.0),
];

impl Size {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        match value {
            Value::String(name) => SIZE_PRESETS
                .iter()
                .find(|(preset, _, _)| preset == name)
                .map(|&(_, w, h)| Size {
                    width: Length::mm(w),
                    height: Length::mm(h),
                })
                .ok_or_else(|| {
                    let names: Vec<_> = SIZE_PRESETS.iter().map(|(n, _, _)| *n).collect();
                    format!("unknown size preset `{name}` (known: {})", names.join(", "))
                }),
            Value::Object(map) => {
                let dim = |key: &str| -> Result<Length, String> {
                    match map.get(key) {
                        Some(Value::String(s)) => s.parse(),
                        Some(_) => Err(format!("`{key}` must be a length string such as \"63mm\"")),
                        None => Err(format!("size is missing `{key}`")),
                    }
                };
                if let Some(extra) = map.keys().find(|k| *k != "width" && *k != "height") {
                    return Err(format!(
                        "unknown size key `{extra}` (expected width, height)"
                    ));
                }
                Ok(Size {
                    width: dim("width")?,
                    height: dim("height")?,
                })
            }
            _ => Err("size must be a preset name or {width, height}".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!("63mm".parse::<Length>().unwrap().as_mm(), 63.0);
        assert_eq!("2.5in".parse::<Length>().unwrap().as_mm(), 63.5);
        assert_eq!("0.5cm".parse::<Length>().unwrap().as_mm(), 5.0);
        assert!("63".parse::<Length>().is_err());
        assert!("63px".parse::<Length>().is_err());
        assert!("-1mm".parse::<Length>().is_err());
    }

    #[test]
    fn resolves_sizes() {
        let poker = Size::from_value(&Value::from("poker")).unwrap();
        assert_eq!(poker.width.as_mm(), 63.0);
        let custom =
            Size::from_value(&serde_json::json!({"width": "50mm", "height": "3in"})).unwrap();
        assert!((custom.height.as_mm() - 76.2).abs() < 1e-9);
        assert!(Size::from_value(&Value::from("bridge")).is_err());
    }
}
