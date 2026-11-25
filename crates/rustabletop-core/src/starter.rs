use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::project::Paper;

const FILES: &[(&str, &str)] = &[
    ("game.yaml", include_str!("../starter/game.yaml")),
    (
        "data/cards.yaml",
        include_str!("../starter/data/cards.yaml"),
    ),
    (
        "data/markers.yaml",
        include_str!("../starter/data/markers.yaml"),
    ),
    (
        "templates/card.typ",
        include_str!("../starter/templates/card.typ"),
    ),
    (
        "templates/marker.typ",
        include_str!("../starter/templates/marker.typ"),
    ),
    (
        "templates/board.typ",
        include_str!("../starter/templates/board.typ"),
    ),
    ("icons/sun.svg", include_str!("../starter/icons/sun.svg")),
    ("icons/moon.svg", include_str!("../starter/icons/moon.svg")),
    (".gitignore", include_str!("../starter/gitignore")),
];

pub fn new_project(dir: &Path, paper: Paper) -> Result<Vec<String>> {
    if dir.exists() {
        let mut entries =
            fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))?;
        if entries.next().is_some() {
            bail!(
                "{} is not empty: `rustabletop new` never overwrites files",
                dir.display()
            );
        }
    }
    let game = game_name(dir);
    let paper = match paper {
        Paper::A4 => "a4",
        Paper::Letter => "letter",
    };
    let mut written = Vec::new();
    for (rel, text) in FILES {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        let text = text.replace("{{game}}", &game).replace("{{paper}}", paper);
        fs::write(&path, text).with_context(|| format!("cannot write {}", path.display()))?;
        written.push(path.display().to_string());
    }
    written.extend(crate::skill::install_skill(dir)?.files);
    Ok(written)
}

fn game_name(dir: &Path) -> String {
    let name: String = dir
        .canonicalize()
        .ok()
        .as_deref()
        .unwrap_or(dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let name = name
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if name.is_empty() {
        "my-game".into()
    } else {
        name
    }
}
