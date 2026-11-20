use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, IntoValue, Str};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

pub struct ProjectWorld {
    root: PathBuf,
    main: FileId,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    sources: Mutex<HashMap<FileId, Source>>,
}

impl ProjectWorld {
    pub fn new(
        root: impl Into<PathBuf>,
        entry: impl AsRef<Path>,
        font_dirs: &[PathBuf],
        inputs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self> {
        let root = root
            .into()
            .canonicalize()
            .context("project root not found")?;
        let entry = root.join(entry.as_ref());
        let vpath = VirtualPath::virtualize(&root, &entry)
            .map_err(|e| anyhow::anyhow!("entry outside project root: {e:?}"))?;
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

        let inputs: Dict = inputs
            .into_iter()
            .map(|(k, v)| (Str::from(k), Str::from(v).into_value()))
            .collect();
        let library = Library::builder().with_inputs(inputs).build();

        let fonts = load_fonts(font_dirs)?;
        let book = FontBook::from_infos(fonts.iter().map(|f| f.info().clone()));

        Ok(Self {
            root,
            main,
            library: LazyHash::new(library),
            book: LazyHash::new(book),
            fonts,
            sources: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn realize(&self, id: FileId) -> FileResult<PathBuf> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(FileError::Other(Some(
                "packages are not supported yet".into(),
            )));
        }
        id.vpath()
            .realize(&self.root)
            .map_err(|_| FileError::AccessDenied)
    }
}

fn load_fonts(dirs: &[PathBuf]) -> Result<Vec<Font>> {
    let mut fonts: Vec<Font> = typst_assets::fonts()
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();

    for dir in dirs {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)
            .with_context(|| format!("cannot read font dir {}", dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                matches!(
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(str::to_ascii_lowercase)
                        .as_deref(),
                    Some("ttf" | "otf" | "ttc" | "otc")
                )
            })
            .collect();
        paths.sort();
        for path in paths {
            let data = fs::read(&path)?;
            fonts.extend(Font::iter(Bytes::new(data)));
        }
    }
    Ok(fonts)
}

impl World for ProjectWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let mut cache = self.sources.lock().unwrap();
        if let Some(source) = cache.get(&id) {
            return Ok(source.clone());
        }
        let path = self.realize(id)?;
        let text = fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
        let source = Source::new(id, text);
        cache.insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let path = self.realize(id)?;
        fs::read(&path)
            .map(Bytes::new)
            .map_err(|e| FileError::from_io(e, &path))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}
