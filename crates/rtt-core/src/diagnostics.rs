use serde::Serialize;
use typst::World;
use typst::WorldExt;
use typst::diag::{Severity as TypstSeverity, SourceDiagnostic};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

impl Location {
    pub fn new(file: impl Into<String>, line: usize, column: Option<usize>) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Subject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            location: None,
            subject: None,
            hints: Vec::new(),
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(code, message)
        }
    }

    pub fn at(mut self, location: Option<Location>) -> Self {
        self.location = location;
        self
    }

    pub fn about(mut self, subject: Subject) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    pub fn hints(mut self, hints: impl IntoIterator<Item = String>) -> Self {
        self.hints.extend(hints);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn from_typst(world: &dyn World, diag: &SourceDiagnostic) -> Self {
        let severity = match diag.severity {
            TypstSeverity::Error => Severity::Error,
            TypstSeverity::Warning => Severity::Warning,
        };
        Self {
            code: "typst",
            severity,
            message: diag.message.to_string(),
            location: locate(world, diag),
            subject: None,
            hints: diag.hints.iter().map(|h| h.v.to_string()).collect(),
        }
    }
}

fn locate(world: &dyn World, diag: &SourceDiagnostic) -> Option<Location> {
    let id = diag.span.id()?;
    let range = world.range(diag.span)?;
    let source = world.source(id).ok()?;
    let (line, column) = source.lines().byte_to_line_column(range.start)?;
    Some(Location::new(
        id.vpath().get_without_slash(),
        line + 1,
        Some(column + 1),
    ))
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{level}[{}]: {}", self.code, self.message)?;
        if let Some(loc) = &self.location {
            write!(f, "\n  --> {}:{}", loc.file, loc.line)?;
            if let Some(col) = loc.column {
                write!(f, ":{col}")?;
            }
        }
        if let Some(subject) = &self.subject {
            let path: Vec<&str> = [&subject.component, &subject.instance, &subject.field]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            if !path.is_empty() {
                write!(f, "\n  at: {}", path.join(" / "))?;
            }
        }
        for hint in &self.hints {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}
