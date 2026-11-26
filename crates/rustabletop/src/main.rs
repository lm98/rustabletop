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
    /// Create a working starter project (card, token, board) in a new directory
    New(NewArgs),
    /// Describe a project: components, shapes, print options, effective schemas
    Schema(SchemaArgs),
    /// Validate a project's manifest, schemas and data
    Check(CheckArgs),
    /// Render one instance to PNG, one image per face
    Render(RenderArgs),
    /// Check, then render the print-and-play PDF and one PNG per face
    Build(BuildArgs),
    /// Export the rustabletop skill for claude.ai, or pin a copy for Claude Code
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// Compile a raw Typst file (low-level, used by spikes)
    #[command(hide = true)]
    Typst(TypstArgs),
}

#[derive(Subcommand)]
enum SkillCommand {
    /// Write the skill as a zip to upload to claude.ai (Customize > Skills)
    Export(SkillExportArgs),
    /// Write the skill into <DIR>/.claude/skills/rustabletop/, replacing other versions
    Install(SkillInstallArgs),
}

#[derive(clap::Args)]
struct SkillExportArgs {
    /// Zip file to write [default: rustabletop-skill-<version>.zip, unless --dir is given]
    #[arg(long)]
    zip: Option<PathBuf>,
    /// Also write the skill folder as <DIR>/rustabletop/
    #[arg(long)]
    dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct SkillInstallArgs {
    #[arg(long, default_value = ".")]
    dir: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum PaperArg {
    A4,
    Letter,
}

#[derive(clap::Args)]
struct NewArgs {
    dir: PathBuf,
    #[arg(long, value_enum, default_value = "a4")]
    paper: PaperArg,
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct SchemaArgs {
    #[arg(default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(clap::Args)]
struct RenderArgs {
    /// `<component>/<instance>`, or `<component>` when it has a single instance
    target: String,
    #[arg(default_value = ".")]
    project: PathBuf,
    /// Render only this face [default: all faces]
    #[arg(long)]
    face: Option<String>,
    /// Output directory [default: <PROJECT>/out/render]
    #[arg(long, short)]
    out: Option<PathBuf>,
    #[arg(long, default_value_t = 150.0)]
    ppi: f32,
    /// Draft mode: `ctx.draft` is true and `art` draws outlines only
    #[arg(long)]
    draft: bool,
    #[arg(long)]
    json: bool,
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
    /// Draft mode: `ctx.draft` is true, `art` draws outlines only, the PDF ends in `-draft`
    #[arg(long)]
    draft: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Pdf,
    Png,
}

#[derive(clap::Args)]
struct TypstArgs {
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
        Command::New(args) => new(&args),
        Command::Schema(args) => schema(&args),
        Command::Check(args) => check(&args),
        Command::Render(args) => render(&args),
        Command::Build(args) => build(&args),
        Command::Skill { command } => match command {
            SkillCommand::Export(args) => skill_export(&args),
            SkillCommand::Install(args) => skill_install(&args),
        },
        Command::Typst(args) => match typst(&args) {
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

fn new(args: &NewArgs) -> ExitCode {
    let paper = match args.paper {
        PaperArg::A4 => rustabletop_core::Paper::A4,
        PaperArg::Letter => rustabletop_core::Paper::Letter,
    };
    match rustabletop_core::new_project(&args.dir, paper) {
        Ok(files) => {
            if args.json {
                let report = serde_json::json!({ "ok": true, "files": files });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report is serializable")
                );
            } else {
                for file in &files {
                    println!("{file}");
                }
                println!(
                    "ok: created {} ({} files); next: rustabletop check {0}",
                    args.dir.display(),
                    files.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if args.json {
                let report = serde_json::json!({ "ok": false, "error": format!("{e:#}") });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report is serializable")
                );
            } else {
                eprintln!("error: {e:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn skill_export(args: &SkillExportArgs) -> ExitCode {
    let zip = match (&args.zip, &args.dir) {
        (None, None) => Some(PathBuf::from(rustabletop_core::default_skill_zip())),
        (zip, _) => zip.clone(),
    };
    match rustabletop_core::export_skill(zip.as_deref(), args.dir.as_deref()) {
        Ok(exported) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&exported).expect("report is serializable")
                );
            } else {
                for path in exported.zip.iter().chain(&exported.dir) {
                    println!("{path}");
                }
                println!("ok: exported skill {}", exported.version);
                if exported.zip.is_some() {
                    println!("upload the zip to claude.ai: Customize > Skills");
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if args.json {
                let report = serde_json::json!({ "ok": false, "error": format!("{e:#}") });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("report is serializable")
                );
            } else {
                eprintln!("error: {e:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn skill_install(args: &SkillInstallArgs) -> ExitCode {
    match rustabletop_core::install_skill(&args.dir) {
        Ok(installed) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&installed).expect("report is serializable")
                );
            } else {
                let (version, dir) = (installed.version, &installed.dir);
                match installed.status {
                    rustabletop_core::SkillStatus::Created => {
                        println!("ok: installed skill {version} in {dir}");
                    }
                    rustabletop_core::SkillStatus::Unchanged => {
                        println!("ok: skill {version} already installed in {dir}");
                    }
                    rustabletop_core::SkillStatus::Updated => {
                        println!("ok: replaced a different skill version with {version} in {dir}");
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn schema(args: &SchemaArgs) -> ExitCode {
    let description = rustabletop_core::describe(&args.project);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&description).expect("report is serializable")
        );
    } else {
        for d in &description.diagnostics {
            eprintln!("{d}\n");
        }
        print!("{}", description.summary());
        if !description.ok {
            let errors = description
                .diagnostics
                .iter()
                .filter(|d| d.is_error())
                .count();
            println!("\nincomplete: {errors} error(s), run `rustabletop check` for details");
        }
    }
    if description.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
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
    let options = rustabletop_core::BuildOptions {
        out,
        ppi: args.ppi,
        draft: args.draft,
    };
    let built = match rustabletop_core::build(&args.project, &options) {
        Ok(built) => built,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(2);
        }
    };
    report_built(&built, args.json)
}

fn render(args: &RenderArgs) -> ExitCode {
    let options = rustabletop_core::RenderOptions {
        out: args
            .out
            .clone()
            .unwrap_or_else(|| args.project.join("out").join("render")),
        ppi: args.ppi,
        face: args.face.clone(),
        draft: args.draft,
    };
    match rustabletop_core::render(&args.project, &args.target, &options) {
        Ok(built) => report_built(&built, args.json),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn report_built(built: &rustabletop_core::Built, json: bool) -> ExitCode {
    if json {
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
            for artifact in built.artifacts.iter().filter(|a| a.ends_with(".pdf")) {
                println!("{artifact}");
            }
            if built.artifacts.len() <= 4 {
                for artifact in built.artifacts.iter().filter(|a| a.ends_with(".png")) {
                    println!("{artifact}");
                }
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

fn typst(args: &TypstArgs) -> Result<Report> {
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
