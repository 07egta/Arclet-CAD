//! Keep failed browser exports visible and retryable without duplicating their VFS bytes.

use std::collections::VecDeque;

use super::MAX_BROWSER_FILE_BYTES;

const MAX_PENDING: usize = 32;

struct PendingDownload {
    path: String,
    retry_at: Option<f64>,
}

#[derive(Default)]
pub(super) struct PendingDownloads {
    entries: VecDeque<PendingDownload>,
    overflowed: bool,
}

impl PendingDownloads {
    pub(super) fn failed(&mut self, path: String, bytes: usize, now: f64) {
        self.completed(&path);
        if self.entries.len() < MAX_PENDING {
            let retry_at = (bytes <= MAX_BROWSER_FILE_BYTES && now.is_finite()).then_some(now + 60_000.0);
            self.entries.push_back(PendingDownload { path, retry_at });
        } else {
            // The caller keeps the bytes in the VFS. Losing a queue slot must never make
            // that unexported document look saved.
            self.overflowed = true;
        }
    }

    pub(super) fn completed(&mut self, path: &str) {
        self.entries.retain(|entry| entry.path != path);
    }

    pub(super) fn is_unsaved(&self) -> bool {
        self.overflowed || !self.entries.is_empty()
    }

    pub(super) fn due(&self, now: f64) -> Option<&str> {
        self.entries.iter().find(|entry| entry.retry_at.is_some_and(|at| at <= now)).map(|entry| entry.path.as_str())
    }

    pub(super) fn next_retry(&self) -> Option<f64> {
        self.entries.iter().filter_map(|entry| entry.retry_at).reduce(f64::min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_export_remains_unsaved_and_retries_the_latest_path_once() {
        let mut queue = PendingDownloads::default();
        queue.failed("design.sc".into(), 100, 0.0);
        assert!(queue.is_unsaved());
        assert_eq!(queue.due(59_999.0), None);
        assert_eq!(queue.due(60_000.0), Some("design.sc"));
        queue.failed("design.sc".into(), 200, 10_000.0);
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.next_retry(), Some(70_000.0));
        assert_eq!(queue.due(60_000.0), None);
        queue.completed("design.sc");
        assert!(!queue.is_unsaved());
        assert_eq!(queue.due(100_000.0), None);
    }

    #[test]
    fn oversized_download_is_retained_without_futile_automatic_retry() {
        let mut queue = PendingDownloads::default();
        queue.failed("large.sc".into(), MAX_BROWSER_FILE_BYTES + 1, 0.0);
        assert!(queue.is_unsaved());
        assert_eq!(queue.due(f64::MAX), None);
        assert_eq!(queue.next_retry(), None);
        queue.failed("large.sc".into(), 100, 0.0);
        assert_eq!(queue.due(60_000.0), Some("large.sc"));
    }

    #[test]
    fn queue_overflow_is_bounded_and_never_falsely_reports_saved() {
        let mut queue = PendingDownloads::default();
        for index in 0..=MAX_PENDING {
            queue.failed(format!("{index}.sc"), 1, 0.0);
        }
        assert_eq!(queue.entries.len(), MAX_PENDING);
        for index in 0..MAX_PENDING {
            queue.completed(&format!("{index}.sc"));
        }
        assert!(queue.is_unsaved());
    }
}
