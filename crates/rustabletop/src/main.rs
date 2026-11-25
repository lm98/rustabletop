use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rustabletop_core::{Diagnostic, ProjectWorld};

#[derive(Parser)]
#[command(
    name = "rustabletop",
    version,
    about = "rustabletop — tabletop components from data"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a project's manifest, schemas and data
    Check(CheckArgs),
    /// Check, then render the print-and-play PDF and one PNG per card face
    Build(BuildArgs),
    /// Compile a raw Typst file (low-level, used by spikes)
    Render(RenderArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    #[arg(default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct BuildArgs {
    #[arg(default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    json: bool,
    /// Output directory [default: <PROJECT>/out]
    #[arg(long, short)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 300.0)]
    ppi: f32,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Pdf,
    Png,
}

#[derive(clap::Args)]
struct RenderArgs {
    entry: PathBuf,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long, short, default_value = "out")]
    out: PathBuf,
    #[arg(long, short, value_enum, default_value = "pdf")]
    format: Format,
    #[arg(long, default_value_t = 300.0)]
    ppi: f32,
    #[arg(long = "font-path")]
    font_paths: Vec<PathBuf>,
    #[arg(long = "input", value_parser = parse_key_value)]
    inputs: Vec<(String, String)>,
    #[arg(long)]
    json: bool,
}

fn parse_key_value(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .ok_or_else(|| format!("expected key=value, got `{s}`"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check(args) => check(&args),
        Command::Build(args) => build(&args),
        Command::Render(args) => match render(&args) {
            Ok(report) => {
                emit(&report, args.json);
                if report.ok {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::from(2)
            }
        },
    }
}

#[derive(serde::Serialize)]
struct CheckReport {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<Summary>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(serde::Serialize)]
struct Summary {
    game: String,
    components: Vec<ComponentSummary>,
}

#[derive(serde::Serialize)]
struct ComponentSummary {
    id: String,
    instances: usize,
    copies: u64,
}

fn check(args: &CheckArgs) -> ExitCode {
    let checked = rustabletop_core::check(&args.project);
    let ok = checked.ok();
    let summary = checked.project.as_ref().map(|p| Summary {
        game: p.game.clone(),
        components: p
            .components
            .iter()
            .map(|c| ComponentSummary {
                id: c.id.clone(),
                instances: c.instances.len(),
                copies: c.total_qty(),
            })
            .collect(),
    });
    let report = CheckReport {
        ok,
        summary,
        diagnostics: checked.diagnostics,
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report is serializable")
        );
    } else {
        for d in &report.diagnostics {
            eprintln!("{d}\n");
        }
        let errors = report.diagnostics.iter().filter(|d| d.is_error()).count();
        let warnings = report.diagnostics.len() - errors;
        match &report.summary {
            Some(s) => {
                let parts: Vec<String> = s
                    .components
                    .iter()
                    .map(|c| format!("{}: {} instances, {} copies", c.id, c.instances, c.copies))
                    .collect();
                println!(
                    "ok: {} ({}), {warnings} warning(s)",
                    s.game,
                    parts.join("; ")
                );
            }
            None => println!("failed: {errors} error(s), {warnings} warning(s)"),
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn build(args: &BuildArgs) -> ExitCode {
    let out = args.out.clone().unwrap_or_else(|| args.project.join("out"));
    let options = rustabletop_core::BuildOptions { out, ppi: args.ppi };
    let built = match rustabletop_core::build(&args.project, &options) {
        Ok(built) => built,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(2);
        }
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&built).expect("report is serializable")
        );
    } else {
        for d in &built.diagnostics {
            eprintln!("{d}\n");
        }
        let errors = built.diagnostics.iter().filter(|d| d.is_error()).count();
        let warnings = built.diagnostics.len() - errors;
        if built.ok {
            if let Some(pdf) = built.artifacts.first() {
                println!("{pdf}");
            }
            println!(
                "ok: {} artifact(s), {warnings} warning(s)",
                built.artifacts.len()
            );
        } else {
            println!("failed: {errors} error(s), {warnings} warning(s)");
        }
    }
    if built.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[derive(serde::Serialize)]
struct Report {
    ok: bool,
    artifacts: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

fn render(args: &RenderArgs) -> Result<Report> {
    let world = ProjectWorld::new(
        &args.root,
        &args.entry,
        &args.font_paths,
        args.inputs.clone(),
    )?;
    let compiled = match rustabletop_core::compile(&world) {
        Ok(c) => c,
        Err(diagnostics) => {
            return Ok(Report {
                ok: false,
                artifacts: vec![],
                diagnostics,
            });
        }
    };

    fs::create_dir_all(&args.out)
        .with_context(|| format!("cannot create {}", args.out.display()))?;
    let stem = args
        .entry
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let mut artifacts = Vec::new();

    match args.format {
        Format::Pdf => {
            let bytes = match rustabletop_core::export_pdf(&world, &compiled.document) {
                Ok(b) => b,
                Err(diagnostics) => {
                    return Ok(Report {
                        ok: false,
                        artifacts,
                        diagnostics,
                    });
                }
            };
            let path = args.out.join(format!("{stem}.pdf"));
            fs::write(&path, bytes)?;
            artifacts.push(path.display().to_string());
        }
        Format::Png => {
            for (i, png) in rustabletop_core::export_pngs(&compiled.document, args.ppi)?
                .into_iter()
                .enumerate()
            {
                let path = args.out.join(format!("{stem}-{:03}.png", i + 1));
                fs::write(&path, png)?;
                artifacts.push(path.display().to_string());
            }
        }
    }

    Ok(Report {
        ok: true,
        artifacts,
        diagnostics: compiled.warnings,
    })
}

fn emit(report: &Report, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).expect("report is serializable")
        );
        return;
    }
    for d in &report.diagnostics {
        eprintln!("{d}");
    }
    for a in &report.artifacts {
        println!("{a}");
    }
}
