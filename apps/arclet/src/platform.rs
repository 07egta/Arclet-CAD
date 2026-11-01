//! Small host services for Arclet. Native files use async OS dialogs; browser files are
//! explicit uploads/downloads over the engine's memory VFS, without background networking.

use solidcraft_ui_egui::Services;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod paths;
#[cfg(any(target_arch = "wasm32", test))]
mod recovery;
#[cfg(target_arch = "wasm32")]
mod web;

/// Arclet's own per-user native data folder. No other host's override or legacy
/// migration is consulted, and choosing this path does not create a directory.
#[cfg(not(target_arch = "wasm32"))]
pub fn data_dir() -> Option<std::path::PathBuf> {
    paths::data_dir()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn crash_dir() -> Option<std::path::PathBuf> {
    data_dir().map(|dir| dir.join("crashes"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recovery_dir() -> Option<std::path::PathBuf> {
    data_dir().map(|dir| dir.join("recovery"))
}

/// Install this host's services in the CAD workspace during app construction.
pub fn services(ctx: &egui::Context) -> Services {
    #[cfg(not(target_arch = "wasm32"))]
    {
        native::services(ctx)
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::services(ctx)
    }
}

/// Call after workspace logic each frame. Errors belong in the app's visible status line.
pub fn poll(ctx: &egui::Context) -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctx;
        Vec::new()
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::poll(ctx)
    }
}

/// A browser write has not yet produced a download. Keep the document visibly unsaved.
pub fn has_unsaved_downloads() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::has_unsaved_downloads()
    }
}

/// Ask the browser to warn before leaving a document with unsaved work. Native closing is
/// handled by the app's own document dialog. This does not persist data or prevent a crash.
pub fn set_unsaved(unsaved: bool) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = unsaved;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::set_unsaved(unsaved)
    }
}

/// A bounded basename, never a browser-controlled directory or hidden path.
fn file_name(name: &str) -> String {
    let leaf = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let mut result = String::new();
    for c in leaf.chars() {
        let c = if c.is_control() || matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c };
        if result.len() + c.len_utf8() > 200 {
            break;
        }
        result.push(c);
    }
    let result = result.trim().trim_start_matches('.').trim_end_matches('.');
    if result.is_empty() { "Untitled.sc".into() } else { result.into() }
}

#[cfg(any(target_arch = "wasm32", test))]
const MAX_BROWSER_FILE_BYTES: usize = 64 * 1024 * 1024;

#[cfg(any(target_arch = "wasm32", test))]
fn validate_upload(size: f64, existing: &[(String, u64)]) -> Result<(), String> {
    if !size.is_finite() || size < 0.0 || size > MAX_BROWSER_FILE_BYTES as f64 {
        return Err("Browser uploads are limited to 64 MiB per file".into());
    }
    let total = existing.iter().try_fold(size as u64, |total, (_, bytes)| total.checked_add(*bytes)).ok_or("Browser file capacity exceeded")?;
    if existing.len() >= 32 || total > 128 * 1024 * 1024 {
        return Err("Browser file capacity reached; download your work and reload before opening more files".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_names_are_bounded_basenames() {
        assert_eq!(file_name("../../part.step"), "part.step");
        assert_eq!(file_name("C:\\temp\\part.stl"), "part.stl");
        assert_eq!(file_name(".."), "Untitled.sc");
        assert_eq!(file_name("bad\nname?.json"), "bad_name_.json");
        let long = file_name(&"界".repeat(1000));
        assert!(long.len() <= 200);
        assert!(!long.is_empty());
    }

    #[test]
    fn uploads_are_rejected_before_reading_when_size_or_cache_exceeds_limits() {
        assert!(validate_upload(1024.0, &[]).is_ok());
        for value in [f64::NAN, f64::INFINITY, -1.0, MAX_BROWSER_FILE_BYTES as f64 + 1.0] {
            assert!(validate_upload(value, &[]).is_err());
        }
        assert!(validate_upload(1.0, &[("full".into(), 128 * 1024 * 1024)]).is_err());
        assert!(validate_upload(1.0, &[("overflow".into(), u64::MAX)]).is_err());
        assert!(validate_upload(0.0, &vec![("file".into(), 0); 32]).is_err());
    }
}
