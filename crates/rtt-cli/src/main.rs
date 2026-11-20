use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rtt_core::{Diagnostic, ProjectWorld};

#[derive(Parser)]
#[command(name = "rtt", version, about = "rustabletop — tabletop components from data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Render(RenderArgs),
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
        Command::Render(args) => match render(&args) {
            Ok(report) => {
                emit(&report, args.json);
                if report.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::from(2)
            }
        },
    }
}

#[derive(serde::Serialize)]
struct Report {
    ok: bool,
    artifacts: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

fn render(args: &RenderArgs) -> Result<Report> {
    let world = ProjectWorld::new(&args.root, &args.entry, &args.font_paths, args.inputs.clone())?;
    let compiled = match rtt_core::compile(&world) {
        Ok(c) => c,
        Err(diagnostics) => return Ok(Report { ok: false, artifacts: vec![], diagnostics }),
    };

    fs::create_dir_all(&args.out).with_context(|| format!("cannot create {}", args.out.display()))?;
    let stem = args.entry.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let mut artifacts = Vec::new();

    match args.format {
        Format::Pdf => {
            let bytes = match rtt_core::export_pdf(&world, &compiled.document) {
                Ok(b) => b,
                Err(diagnostics) => return Ok(Report { ok: false, artifacts, diagnostics }),
            };
            let path = args.out.join(format!("{stem}.pdf"));
            fs::write(&path, bytes)?;
            artifacts.push(path.display().to_string());
        }
        Format::Png => {
            for (i, png) in rtt_core::export_pngs(&compiled.document, args.ppi)?.into_iter().enumerate() {
                let path = args.out.join(format!("{stem}-{:03}.png", i + 1));
                fs::write(&path, png)?;
                artifacts.push(path.display().to_string());
            }
        }
    }

    Ok(Report { ok: true, artifacts, diagnostics: compiled.warnings })
}

fn emit(report: &Report, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(report).expect("report is serializable"));
        return;
    }
    for d in &report.diagnostics {
        eprintln!("{d}");
    }
    for a in &report.artifacts {
        println!("{a}");
    }
}
