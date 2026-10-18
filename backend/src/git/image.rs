//! Reading an image of a repository (a file in the working tree or in a commit) for
//! the image diff. Only a short list of picture formats is served, never more than
//! [`MAX_BYTES`], and the working tree file must really lie inside the repository.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;

use super::{Git, decode};
use crate::error::{AppError, Result};

/// Larger pictures are reported by size only.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub mime: &'static str,
    pub size: u64,
    /// `data:<mime>;base64,...`; None when the picture is larger than [`MAX_BYTES`].
    pub data_url: Option<String>,
}

/// The picture format of a file name (SVG is left out: it can carry scripts and links).
pub fn mime_of(path: &str) -> Option<&'static str> {
    let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        _ => return None,
    })
}

fn image(mime: &'static str, size: u64, bytes: Option<Vec<u8>>) -> Image {
    Image {
        mime,
        size,
        data_url: bytes.map(|b| format!("data:{mime};base64,{}", STANDARD.encode(b))),
    }
}

/// The picture at `path` in commit `rev` (an object id), or in the working tree when
/// `rev` is `None`. `Ok(None)` when the file doesn't exist there (or isn't a file).
pub fn read(repo: &Path, rev: Option<&str>, path: &str) -> Result<Option<Image>> {
    let mime = mime_of(path).ok_or_else(|| AppError::InvalidInput(path.into()))?;
    match rev {
        Some(rev) => read_revision(repo, rev, path, mime),
        None => read_working(repo, path, mime),
    }
}

fn read_working(repo: &Path, path: &str, mime: &'static str) -> Result<Option<Image>> {
    let file = repo.join(path);
    let Ok(meta) = std::fs::symlink_metadata(&file) else {
        return Ok(None);
    };
    if !meta.is_file() {
        return Ok(None);
    }
    // A folder of the path may be a link out of the repository.
    let (Ok(real), Ok(root)) = (std::fs::canonicalize(&file), std::fs::canonicalize(repo)) else {
        return Ok(None);
    };
    if !real.starts_with(&root) {
        return Ok(None);
    }
    if meta.len() > MAX_BYTES {
        return Ok(Some(image(mime, meta.len(), None)));
    }
    Ok(Some(image(mime, meta.len(), Some(std::fs::read(&real)?))))
}

fn read_revision(repo: &Path, rev: &str, path: &str, mime: &'static str) -> Result<Option<Image>> {
    let object = format!("{rev}:{path}");
    let size = Git::new(repo)
        .args(["cat-file", "-s"])
        .arg(&object)
        .ok_codes(&[0, 128])
        .run()?;
    if size.code != 0 {
        return Ok(None);
    }
    let size: u64 = decode(&size.stdout).trim().parse().unwrap_or(u64::MAX);
    if size > MAX_BYTES {
        return Ok(Some(image(mime, size, None)));
    }
    // A folder has a size too; `cat-file blob` refuses it.
    let blob = Git::new(repo)
        .args(["cat-file", "blob"])
        .arg(&object)
        .ok_codes(&[0, 128])
        .run()?;
    Ok((blob.code == 0).then(|| image(mime, size, Some(blob.stdout))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, git_out, repo_with_commit as repo};

    /// A 1x1 PNG.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00,
        0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0, 0x1f, 0x00, 0x05, 0x00, 0x01, 0xff,
        0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn knows_picture_formats_only() {
        assert_eq!(mime_of("a/b.PNG"), Some("image/png"));
        assert_eq!(mime_of("x.jpeg"), Some("image/jpeg"));
        assert_eq!(mime_of("logo.svg"), None);
        assert_eq!(mime_of("notes.txt"), None);
        assert_eq!(mime_of("png"), None);
        assert!(matches!(
            read(Path::new("."), None, "a.txt"),
            Err(AppError::InvalidInput(_))
        ));
    }

    #[test]
    fn reads_working_and_committed_pictures() {
        let dir = repo();
        let p = dir.path();
        std::fs::create_dir(p.join("画像")).unwrap();
        std::fs::write(p.join("画像/logo.png"), PNG).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "add logo"]);
        let head = git_out(p, &["rev-parse", "HEAD"]);
        let before = git_out(p, &["rev-parse", "HEAD~1"]);

        let committed = read(p, Some(&head), "画像/logo.png").unwrap().unwrap();
        assert_eq!(committed.size, PNG.len() as u64);
        assert!(
            committed
                .data_url
                .as_deref()
                .unwrap()
                .starts_with("data:image/png;base64,iVBORw0KGgo")
        );
        assert_eq!(read(p, None, "画像/logo.png").unwrap(), Some(committed));
        // Missing on that side: no picture, no error.
        assert_eq!(read(p, Some(&before), "画像/logo.png").unwrap(), None);
        assert_eq!(read(p, None, "画像/none.png").unwrap(), None);
        // A folder is not a picture.
        std::fs::create_dir(p.join("dir.png")).unwrap();
        assert_eq!(read(p, None, "dir.png").unwrap(), None);
    }

    #[test]
    fn large_pictures_are_reported_by_size() {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("big.bmp"), vec![0u8; MAX_BYTES as usize + 1]).unwrap();
        let img = read(p, None, "big.bmp").unwrap().unwrap();
        assert_eq!((img.size, img.data_url), (MAX_BYTES + 1, None));
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "big"]);
        let head = git_out(p, &["rev-parse", "HEAD"]);
        assert_eq!(read(p, Some(&head), "big.bmp").unwrap().unwrap().data_url, None);
    }
}
