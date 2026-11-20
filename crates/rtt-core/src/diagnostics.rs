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

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
}

impl Diagnostic {
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
            hints: diag.hints.iter().map(|h| h.v.to_string()).collect(),
        }
    }
}

fn locate(world: &dyn World, diag: &SourceDiagnostic) -> Option<Location> {
    let id = diag.span.id()?;
    let range = world.range(diag.span)?;
    let source = world.source(id).ok()?;
    let (line, column) = source.lines().byte_to_line_column(range.start)?;
    Some(Location {
        file: id.vpath().get_without_slash().to_string(),
        line: line + 1,
        column: column + 1,
    })
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{level}[{}]: {}", self.code, self.message)?;
        if let Some(loc) = &self.location {
            write!(f, "\n  --> {}:{}:{}", loc.file, loc.line, loc.column)?;
        }
        for hint in &self.hints {
            write!(f, "\n  hint: {hint}")?;
        }
        Ok(())
    }
}
