use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict, IntoValue, Str};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

pub const STD_PACKAGE: &str = "@rtt/std:0.1.0";
const STD_LIB: &str = include_str!("../typst/std/lib.typ");
const STD_MANIFEST: &str = include_str!("../typst/std/typst.toml");
const ICON_DIR: &str = "/icons/";

pub struct ProjectWorld {
    root: PathBuf,
    main: FileId,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    sources: Mutex<HashMap<FileId, Source>>,
    overlay: HashMap<FileId, Bytes>,
    std: PackageSpec,
    icons: HashMap<String, PathBuf>,
    draft: bool,
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
        Self::with_main(root, vpath, font_dirs, inputs)
    }

    pub fn virtual_main(
        root: impl Into<PathBuf>,
        main: &str,
        font_dirs: &[PathBuf],
    ) -> Result<Self> {
        let root = root
            .into()
            .canonicalize()
            .context("project root not found")?;
        let vpath =
            VirtualPath::new(main).map_err(|e| anyhow::anyhow!("invalid path {main}: {e:?}"))?;
        Self::with_main(root, vpath, font_dirs, [])
    }

    fn with_main(
        root: PathBuf,
        vpath: VirtualPath,
        font_dirs: &[PathBuf],
        inputs: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self> {
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
            overlay: HashMap::new(),
            std: STD_PACKAGE.parse().expect("valid package spec"),
            icons: HashMap::new(),
            draft: false,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn add_file(&mut self, path: &str, bytes: impl Into<Vec<u8>>) -> Result<()> {
        let vpath =
            VirtualPath::new(path).map_err(|e| anyhow::anyhow!("invalid path {path}: {e:?}"))?;
        let id = RootedPath::new(VirtualRoot::Project, vpath).intern();
        self.overlay.insert(id, Bytes::new(bytes.into()));
        Ok(())
    }

    pub fn set_icons(&mut self, icons: &BTreeMap<String, String>) {
        self.icons = icons
            .iter()
            .map(|(name, path)| (icon_file(name, path), self.root.join(path)))
            .collect();
    }

    pub fn set_draft(&mut self, draft: bool) {
        self.draft = draft;
    }

    fn icons_module(&self) -> String {
        let mut names: Vec<&String> = self.icons.keys().collect();
        names.sort();
        let entries: String = names
            .iter()
            .map(|file| {
                let name = file.split('.').next().unwrap_or(file);
                format!("  \"{name}\": \"{ICON_DIR}{file}\",\n")
            })
            .collect();
        if entries.is_empty() {
            "#let icons = (:)\n".into()
        } else {
            format!("#let icons = (\n{entries})\n")
        }
    }

    fn package_file(&self, spec: &PackageSpec, vpath: &VirtualPath) -> FileResult<Bytes> {
        if *spec != self.std {
            return Err(FileError::Other(Some(
                format!("only {STD_PACKAGE} is available, not @{spec}").into(),
            )));
        }
        let path = vpath.get_with_slash();
        match path {
            "/typst.toml" => Ok(Bytes::from_string(STD_MANIFEST)),
            "/lib.typ" => Ok(Bytes::from_string(STD_LIB)),
            "/icons.typ" => Ok(Bytes::from_string(self.icons_module())),
            "/mode.typ" => Ok(Bytes::from_string(format!("#let draft = {}\n", self.draft))),
            _ => {
                let real = path
                    .strip_prefix(ICON_DIR)
                    .and_then(|file| self.icons.get(file))
                    .ok_or_else(|| FileError::NotFound(path.into()))?;
                fs::read(real)
                    .map(Bytes::new)
                    .map_err(|e| FileError::from_io(e, real))
            }
        }
    }
}

fn icon_file(name: &str, path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("svg")
        .to_ascii_lowercase();
    format!("{name}.{extension}")
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
        let bytes = self.file(id)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
        let source = Source::new(id, text.into());
        cache.insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(bytes) = self.overlay.get(&id) {
            return Ok(bytes.clone());
        }
        match id.root() {
            VirtualRoot::Package(spec) => self.package_file(spec, id.vpath()),
            VirtualRoot::Project => {
                let path = id
                    .vpath()
                    .realize(&self.root)
                    .map_err(|_| FileError::AccessDenied)?;
                fs::read(&path)
                    .map(Bytes::new)
                    .map_err(|e| FileError::from_io(e, &path))
            }
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}
