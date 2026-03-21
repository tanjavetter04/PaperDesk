use std::path::{Component, Path, PathBuf};

/// Typst project entry (fixed; not configurable).
pub const MAIN_TYP: &str = "main.typ";

/// Lexically join `relative` to canonical `root` and ensure the result stays under `root`.
pub fn join_under_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err("path must be relative".into());
    }
    let mut out = root.to_path_buf();
    for c in rel.components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("invalid path component".into());
            }
        }
    }
    if !out.starts_with(root) {
        return Err("path escapes project root".into());
    }
    Ok(out)
}
