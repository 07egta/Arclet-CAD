//! Native state belongs to Arclet, independent of the shared engine's application name.

use std::ffi::OsString;
use std::path::PathBuf;

pub(super) fn data_dir() -> Option<PathBuf> {
    select_data_dir(std::env::consts::OS, |key| std::env::var_os(key))
}

fn select_data_dir(target_os: &str, mut read_env: impl FnMut(&str) -> Option<OsString>) -> Option<PathBuf> {
    // An empty or relative XDG path is not a per-user data root. In particular, do
    // not create a recovery folder relative to the directory the app was launched in.
    let mut root = |key| read_env(key).map(PathBuf::from).filter(|path| path.is_absolute());
    match target_os {
        "windows" => root("LOCALAPPDATA").map(|root| root.join("Arclet")),
        "macos" => root("HOME").map(|root| root.join("Library/Application Support/Arclet")),
        _ => root("XDG_DATA_HOME").or_else(|| root("HOME").map(|root| root.join(".local/share"))).map(|root| root.join("arclet")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic absolute roots work on the test host without touching any real
    // user's environment or creating files. Each OS's layout can be checked here.
    fn root() -> PathBuf {
        std::env::temp_dir().join("arclet-path-layout-only")
    }

    #[test]
    fn each_platform_uses_an_arclet_namespace() {
        let root = root();
        for (os, variable, suffix) in
            [("windows", "LOCALAPPDATA", "Arclet"), ("macos", "HOME", "Library/Application Support/Arclet"), ("linux", "XDG_DATA_HOME", "arclet")]
        {
            let path = select_data_dir(os, |key| (key == variable).then(|| root.clone().into_os_string()));
            assert_eq!(path, Some(root.join(suffix)));
        }
    }

    #[test]
    fn invalid_xdg_root_falls_back_to_home_without_using_another_host_override() {
        let home = root();
        for xdg in ["", "relative/data"] {
            let path = select_data_dir("linux", |key| match key {
                "XDG_DATA_HOME" => Some(xdg.into()),
                "HOME" => Some(home.clone().into_os_string()),
                "SOLIDCRAFT_RECOVERY_DIR" | "SOLIDCRAFT_CRASH_DIR" => Some(home.join("another-app").into_os_string()),
                _ => None,
            });
            assert_eq!(path, Some(home.join(".local/share/arclet")));
        }
    }

    #[test]
    fn a_missing_user_root_is_reported_without_a_working_directory_fallback() {
        for os in ["windows", "macos", "linux"] {
            assert_eq!(select_data_dir(os, |_| None), None);
            assert_eq!(select_data_dir(os, |_| Some("relative".into())), None);
        }
    }
}
