//! Typst `World` backed by the project directory (adapted from typst-cli's `world.rs`, Apache-2.0).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{fmt, fs, io, mem};

use chrono::{Datelike, FixedOffset, Local, Utc};
use ecow::{EcoString, eco_format};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Dict};
use typst::syntax::{FileId, Lines, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::download::Downloader;
use typst_kit::fonts::{FontSlot, Fonts};
use typst_kit::package::PackageStorage;
use typst_timing::timed;

use super::noop_progress::NoopProgress;

/// A world that reads sources only from a fixed project root on disk.
pub struct PaperDeskWorld {
    workdir: Option<PathBuf>,
    root: PathBuf,
    main: FileId,
    /// Unsaved editor buffers: compiled as if written to disk (live preview).
    source_overrides: FxHashMap<FileId, EcoString>,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    slots: Mutex<FxHashMap<FileId, FileSlot>>,
    package_storage: PackageStorage,
    now: Now,
}

fn resolve_source_file_id(root: &Path, relative: &str) -> Result<FileId, WorldCreationError> {
    let rel = Path::new(relative);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(WorldCreationError::InvalidMainPath);
    }

    let input_path = root.join(rel);
    let input_path = input_path.canonicalize().map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => WorldCreationError::InputNotFound(input_path),
        _ => WorldCreationError::Io(err),
    })?;

    let vpath = VirtualPath::within_root(&input_path, root).ok_or(WorldCreationError::InputOutsideRoot)?;
    Ok(FileId::new(None, vpath))
}

impl PaperDeskWorld {
    /// `main_relative` is a project-relative path using `/` (e.g. `main.typ`).
    ///
    /// `source_overrides`: project-relative paths and UTF-8 text used instead of on-disk contents
    /// for those files (e.g. the open editor buffer before autosave).
    pub fn new(
        project_root: PathBuf,
        main_relative: &str,
        package_cache: PathBuf,
        source_overrides: Vec<(String, String)>,
    ) -> Result<Self, WorldCreationError> {
        let root = project_root.canonicalize().map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => WorldCreationError::RootNotFound(project_root),
            _ => WorldCreationError::Io(err),
        })?;

        let main = resolve_source_file_id(&root, main_relative)?;

        let mut overrides: FxHashMap<FileId, EcoString> = FxHashMap::default();
        for (rel, text) in source_overrides {
            let rel = rel.replace('\\', "/");
            let id = resolve_source_file_id(&root, rel.trim())?;
            overrides.insert(id, text.into());
        }

        let library = Library::builder().with_inputs(Dict::default()).build();

        let mut fonts = Fonts::searcher();
        fonts.include_system_fonts(true);
        let fonts = fonts.search();

        let downloader = Downloader::new(concat!("paperdesk/", env!("CARGO_PKG_VERSION")));
        let package_storage = PackageStorage::new(Some(package_cache), None, downloader);

        Ok(Self {
            workdir: std::env::current_dir().ok(),
            root,
            main,
            source_overrides: overrides,
            library: LazyHash::new(library),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            slots: Mutex::new(FxHashMap::default()),
            package_storage,
            now: Now::System(OnceLock::new()),
        })
    }

    pub fn main_id(&self) -> FileId {
        self.main
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    #[allow(dead_code)]
    pub fn workdir(&self) -> &Path {
        self.workdir.as_deref().unwrap_or(Path::new("."))
    }

    pub fn reset(&mut self) {
        #[allow(clippy::iter_over_hash_type, reason = "order does not matter")]
        for slot in self.slots.get_mut().values_mut() {
            slot.reset();
        }
        let Now::System(time_lock) = &mut self.now;
        time_lock.take();
    }

    #[track_caller]
    pub fn lookup(&self, id: FileId) -> Lines<String> {
        self.slot(id, |slot| {
            if let Some(source) = slot.source.get() {
                let source = source.as_ref().expect("file is not valid");
                source.lines().clone()
            } else if let Some(bytes) = slot.file.get() {
                let bytes = bytes.as_ref().expect("file is not valid");
                Lines::try_from(bytes).expect("file is not valid utf-8")
            } else {
                panic!("file id does not point to any source file");
            }
        })
    }

    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot) -> T,
    {
        let mut map = self.slots.lock();
        f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
    }
}

impl World for PaperDeskWorld {
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
        let ov = &self.source_overrides;
        self.slot(id, |slot| slot.source(&self.root, &self.package_storage, ov))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let ov = &self.source_overrides;
        self.slot(id, |slot| slot.file(&self.root, &self.package_storage, ov))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let now = match &self.now {
            Now::System(time) => time.get_or_init(Utc::now),
        };

        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

struct FileSlot {
    id: FileId,
    source: SlotCell<Source>,
    file: SlotCell<Bytes>,
}

impl FileSlot {
    fn new(id: FileId) -> Self {
        Self { id, file: SlotCell::new(), source: SlotCell::new() }
    }

    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    fn reset(&mut self) {
        self.source.reset();
        self.file.reset();
    }

    fn source(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
        overrides: &FxHashMap<FileId, EcoString>,
    ) -> FileResult<Source> {
        self.source.get_or_init(
            || load_bytes(self.id, project_root, package_storage, overrides),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    fn file(
        &mut self,
        project_root: &Path,
        package_storage: &PackageStorage,
        overrides: &FxHashMap<FileId, EcoString>,
    ) -> FileResult<Bytes> {
        self.file.get_or_init(
            || load_bytes(self.id, project_root, package_storage, overrides),
            |data, _| Ok(Bytes::new(data)),
        )
    }
}

struct SlotCell<T> {
    data: Option<FileResult<T>>,
    fingerprint: u128,
    accessed: bool,
}

impl<T: Clone> SlotCell<T> {
    fn new() -> Self {
        Self { data: None, fingerprint: 0, accessed: false }
    }

    fn accessed(&self) -> bool {
        self.accessed
    }

    fn reset(&mut self) {
        self.accessed = false;
    }

    fn get(&self) -> Option<&FileResult<T>> {
        self.data.as_ref()
    }

    fn get_or_init(
        &mut self,
        load: impl FnOnce() -> FileResult<Vec<u8>>,
        f: impl FnOnce(Vec<u8>, Option<T>) -> FileResult<T>,
    ) -> FileResult<T> {
        if mem::replace(&mut self.accessed, true) {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let result = timed!("loading file", load());
        let fingerprint = timed!("hashing file", typst::utils::hash128(&result));

        if mem::replace(&mut self.fingerprint, fingerprint) == fingerprint {
            if let Some(data) = &self.data {
                return data.clone();
            }
        }

        let prev = self.data.take().and_then(Result::ok);
        let value = result.and_then(|data| f(data, prev));
        self.data = Some(value.clone());

        value
    }
}

fn system_path(
    project_root: &Path,
    id: FileId,
    package_storage: &PackageStorage,
) -> FileResult<PathBuf> {
    let buf;
    let mut root = project_root;
    if let Some(spec) = id.package() {
        buf = package_storage.prepare_package(spec, &mut NoopProgress)?;
        root = &buf;
    }

    id.vpath().resolve(root).ok_or(FileError::AccessDenied)
}

fn load_bytes(
    id: FileId,
    project_root: &Path,
    package_storage: &PackageStorage,
    overrides: &FxHashMap<FileId, EcoString>,
) -> FileResult<Vec<u8>> {
    if let Some(t) = overrides.get(&id) {
        return Ok(t.as_bytes().to_vec());
    }
    read(id, project_root, package_storage)
}

fn read(
    id: FileId,
    project_root: &Path,
    package_storage: &PackageStorage,
) -> FileResult<Vec<u8>> {
    read_from_disk(&system_path(project_root, id, package_storage)?)
}

fn read_from_disk(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}

enum Now {
    System(OnceLock<chrono::DateTime<Utc>>),
}

#[derive(Debug)]
pub enum WorldCreationError {
    InputNotFound(PathBuf),
    InputOutsideRoot,
    RootNotFound(PathBuf),
    InvalidMainPath,
    Io(io::Error),
}

impl fmt::Display for WorldCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldCreationError::InputNotFound(path) => {
                write!(f, "input file not found (searched at {})", path.display())
            }
            WorldCreationError::InputOutsideRoot => {
                write!(f, "source file must be contained in project root")
            }
            WorldCreationError::RootNotFound(path) => {
                write!(f, "root directory not found (searched at {})", path.display())
            }
            WorldCreationError::InvalidMainPath => {
                write!(f, "main file path must be relative with no parent segments")
            }
            WorldCreationError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<WorldCreationError> for EcoString {
    fn from(err: WorldCreationError) -> Self {
        eco_format!("{err}")
    }
}
