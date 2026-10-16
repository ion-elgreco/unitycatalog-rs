//! Where an installed binary finds its data: `share/unitycatalog-rs` in the
//! prefix of its `bin/` folder. The Python wheels, the release tarballs and
//! the image install the binaries and the web UI this way:
//!
//! ```text
//! <prefix>/bin/uc
//! <prefix>/bin/uc-server
//! <prefix>/share/unitycatalog-rs/ui/index.html
//! ```

use std::path::{Path, PathBuf};

/// The data folder of the binary at `exe`: `<prefix>/share/unitycatalog-rs`
/// for `<prefix>/bin/<binary>` (`<prefix>\Scripts\<binary>` on Windows).
pub fn share_dir_of(exe: &Path) -> Option<PathBuf> {
    Some(
        exe.parent()?
            .parent()?
            .join("share")
            .join("unitycatalog-rs"),
    )
}

/// The data folder of the running binary. Symlinks are resolved first, as
/// `uv tool` and pipx link the commands into another folder.
pub fn share_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    share_dir_of(&std::fs::canonicalize(&exe).unwrap_or(exe))
}

/// The web UI's folder: `dir` (the `ui.dir` setting), else `ui` in
/// [`share_dir`]. `None` when the folder does not exist.
pub fn ui_dir(dir: Option<PathBuf>) -> Option<PathBuf> {
    ui_dir_from(dir, share_dir())
}

fn ui_dir_from(dir: Option<PathBuf>, share: Option<PathBuf>) -> Option<PathBuf> {
    let dir = match dir {
        Some(dir) => dir,
        None => share?.join("ui"),
    };
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_dir_is_next_to_the_bin_folder() {
        assert_eq!(
            share_dir_of(Path::new("/opt/uc/bin/uc-server")),
            Some(PathBuf::from("/opt/uc/share/unitycatalog-rs"))
        );
        assert_eq!(share_dir_of(Path::new("uc-server")), None);
    }

    #[test]
    fn ui_dir_is_the_setting_else_the_installed_ui() {
        let share = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let share = share.path().to_path_buf();
        let installed = share.join("ui");

        // Not installed, and no ui.dir.
        assert_eq!(ui_dir_from(None, Some(share.clone())), None);
        assert_eq!(ui_dir_from(None, None), None);

        std::fs::create_dir(&installed).unwrap();
        assert_eq!(ui_dir_from(None, Some(share.clone())), Some(installed));
        // ui.dir wins; a missing folder is none.
        assert_eq!(
            ui_dir_from(Some(other.path().into()), Some(share.clone())),
            Some(other.path().to_path_buf())
        );
        assert_eq!(
            ui_dir_from(Some(other.path().join("missing")), Some(share)),
            None
        );
    }
}
