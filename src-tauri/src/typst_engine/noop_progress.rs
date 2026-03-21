use typst_kit::download::{DownloadState, Progress};

/// Silent package download progress (no terminal output).
pub struct NoopProgress;

impl Progress for NoopProgress {
    fn print_start(&mut self) {}

    fn print_progress(&mut self, _state: &DownloadState) {}

    fn print_finish(&mut self, _state: &DownloadState) {}
}
