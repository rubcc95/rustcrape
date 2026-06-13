pub trait ProgressReporter: Send + Sync {
    fn on_status(&self, _status: &str) {}
    fn on_error(&self, _error: &str) {}
    fn on_quadrant_progress(
        &self,
        _current: usize,
        _total: usize,
        _lat: f64,
        _lng: f64,
        _msg: &str,
    ) {
    }
    fn on_quadrant_complete(
        &self,
        _current: usize,
        _total: usize,
        _lat: f64,
        _lng: f64,
        _count: usize,
    ) {
    }
}

#[derive(Clone, Default)]
pub struct NoopProgress;

impl ProgressReporter for NoopProgress {}
