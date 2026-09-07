use super::{IssueKind, MAX_FILE_BYTES};
use lsp_types::Uri;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) fn file_path(uri: &Uri) -> Result<PathBuf, IssueKind> {
    let url = url::Url::parse(uri.as_str()).map_err(|_| IssueKind::InvalidUri)?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(IssueKind::InvalidUri);
    }
    url.to_file_path().map_err(|()| IssueKind::InvalidUri)
}

pub(super) fn file_uri(path: &Path) -> Result<Uri, IssueKind> {
    url::Url::from_file_path(path)
        .map_err(|()| IssueKind::InvalidUri)?
        .as_str()
        .parse()
        .map_err(|_| IssueKind::InvalidUri)
}

pub(super) fn excluded(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        [".git", ".iris", "target", "node_modules"]
            .iter()
            .any(|excluded| name.eq_ignore_ascii_case(excluded))
    })
}

pub(super) fn check_path(root: &Path, path: &Path) -> Result<(), IssueKind> {
    let relative = path.strip_prefix(root).map_err(|_| IssueKind::UnsafePath)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(IssueKind::UnsafePath);
        }
        current.push(component);
        if excluded(&current) {
            return Err(IssueKind::UnsafePath);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(IssueKind::UnsafePath),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(IssueKind::Io(error.kind())),
        }
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => Err(IssueKind::NonRegular),
        Ok(_) => {
            let actual = path
                .canonicalize()
                .map_err(|error| IssueKind::Io(error.kind()))?;
            let root = root
                .canonicalize()
                .map_err(|error| IssueKind::Io(error.kind()))?;
            if actual.starts_with(root) {
                Ok(())
            } else {
                Err(IssueKind::UnsafePath)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(IssueKind::Io(error.kind())),
    }
}

pub(super) fn read_text(path: &Path, remaining: &mut usize) -> Result<String, IssueKind> {
    let metadata = fs::symlink_metadata(path).map_err(|error| IssueKind::Io(error.kind()))?;
    if metadata.file_type().is_symlink() {
        return Err(IssueKind::UnsafePath);
    }
    if !metadata.is_file() {
        return Err(IssueKind::NonRegular);
    }
    let limit = MAX_FILE_BYTES.min(*remaining);
    if metadata.len() > u64::try_from(limit).map_err(|_| IssueKind::FileBytes)? {
        return Err(if limit == MAX_FILE_BYTES {
            IssueKind::FileBytes
        } else {
            IssueKind::TotalBytes
        });
    }
    let file = fs::File::open(path).map_err(|error| IssueKind::Io(error.kind()))?;
    if !file
        .metadata()
        .map_err(|error| IssueKind::Io(error.kind()))?
        .is_file()
    {
        return Err(IssueKind::NonRegular);
    }
    let mut bytes = Vec::new();
    file.take(u64::try_from(limit + 1).map_err(|_| IssueKind::FileBytes)?)
        .read_to_end(&mut bytes)
        .map_err(|error| IssueKind::Io(error.kind()))?;
    *remaining = remaining.saturating_sub(bytes.len());
    if bytes.len() > limit {
        return Err(if limit == MAX_FILE_BYTES {
            IssueKind::FileBytes
        } else {
            IssueKind::TotalBytes
        });
    }
    String::from_utf8(bytes).map_err(|_| IssueKind::InvalidUtf8)
}
