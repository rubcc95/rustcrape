use crate::types::Coincidence;

pub trait Verboser: Send + Sync {
    fn generating_bounds(&self, checked: usize, valid: usize, total: usize);
    fn connecting_db(&self);
    fn creating_db(&self);
    fn verifying_db(&self);
    fn creating_tables(&self);
    fn rate_limit_wait(&self, wait: std::time::Duration);
    fn obtaining_bound(&self);
    fn already_claimed_bound(&self, lat: f32, lng: f32);
    fn released_bound(&self);
    fn finished(&self);
    fn opening_browser(&self, lat: f32, lng: f32);
    fn closing_browser(&self);
    fn scraping_start(&self);
    fn accepting_cookies(&self);
    fn searching_coincidences(&self);
    fn found_single_coincidence(&self);
    fn found_multiple_coincidences(&self);
    fn processed_coincidence(&self, name: &str, count: usize);
    fn writing_coincidences(&self, output: &[Coincidence]);
    fn written_coincidences(&self, count: i32);
    fn warn(&self, msg: &str);
    fn vpn_rotating(&self) {}
    fn vpn_rotated(&self) {}
    fn vpn_not_available(&self) {}
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
pub struct DebugProgress;

impl Verboser for DebugProgress {
    #[inline(always)]
    fn generating_bounds(&self, _: usize, _: usize, _: usize) {}
    #[inline(always)]
    fn connecting_db(&self) {}
    #[inline(always)]
    fn creating_db(&self) {}
    #[inline(always)]
    fn verifying_db(&self) {}
    #[inline(always)]
    fn creating_tables(&self) {}
    #[inline(always)]
    fn rate_limit_wait(&self, _: std::time::Duration) {}
    #[inline(always)]
    fn obtaining_bound(&self) {}
    #[inline(always)]
    fn released_bound(&self) {}
    #[inline(always)]
    fn finished(&self) {}
    #[inline(always)]
    fn already_claimed_bound(&self, _: f32, _: f32) {}
    #[inline(always)]
    fn opening_browser(&self, _: f32, _: f32) {}
    #[inline(always)]
    fn closing_browser(&self) {}
    #[inline(always)]
    fn scraping_start(&self) {}
    #[inline(always)]
    fn accepting_cookies(&self) {}
    #[inline(always)]
    fn searching_coincidences(&self) {}
    #[inline(always)]
    fn found_single_coincidence(&self) {}
    #[inline(always)]
    fn found_multiple_coincidences(&self) {}
    #[inline(always)]
    fn processed_coincidence(&self, _: &str, _: usize) {}
    #[inline(always)]
    fn writing_coincidences(&self, _: &[Coincidence]) {}
    #[inline(always)]
    fn written_coincidences(&self, _: i32) {}
    #[inline(always)]
    fn warn(&self, _: &str) {}
    fn vpn_rotating(&self) {}
    fn vpn_rotated(&self) {}
    fn vpn_not_available(&self) {}
}

#[cfg(debug_assertions)]
#[derive(Clone, Default)]
pub struct DebugVerboser;

#[cfg(debug_assertions)]
impl Verboser for DebugVerboser {
    fn generating_bounds(&self, checked: usize, valid: usize, total: usize) {
        eprintln!("Generating bounds: checked = {checked}, valid = {valid}, total = {total}",);
    }
    fn connecting_db(&self) {
        eprintln!("Connecting to database...");
    }
    fn creating_db(&self) {
        eprintln!("Creating database...");
    }
    fn verifying_db(&self) {
        eprintln!("Verifying database...");
    }
    fn creating_tables(&self) {
        eprintln!("Creating tables...");
    }
    fn rate_limit_wait(&self, wait: std::time::Duration) {
        let total_secs = wait.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        eprintln!("Rate limit reached. Waiting for {mins}:{secs:02} minutes...");
    }
    fn obtaining_bound(&self) {
        eprintln!("Obtaining bound to process...");
    }
    fn finished(&self) {
        eprintln!("Finished processing all bounds.");
    }
    fn already_claimed_bound(&self, lat: f32, lng: f32) {
        eprintln!("Claimed bound at ({lat}, {lng})");
    }
    fn released_bound(&self) {
        eprintln!("Released bound");
    }
    fn opening_browser(&self, lat: f32, lng: f32) {
        eprintln!("Opening browser at ({lat}, {lng})...");
    }
    fn closing_browser(&self) {
        eprintln!("Closing browser...");
    }
    fn scraping_start(&self) {
        eprintln!("Scraping...");
    }
    fn accepting_cookies(&self) {
        eprintln!("Accepting cookies...");
    }
    fn searching_coincidences(&self) {
        eprintln!("Searching coincidences...");
    }
    fn found_single_coincidence(&self) {
        eprintln!("Found single coincidence");
    }
    fn found_multiple_coincidences(&self) {
        eprintln!("Found multiple coincidences");
    }
    fn processed_coincidence(&self, name: &str, count: usize) {
        eprintln!("Processed {name} ({count} items)");
    }
    fn writing_coincidences(&self, output: &[Coincidence]) {
        eprintln!("Writing {} coincidences to database...", output.len());
    }
    fn written_coincidences(&self, count: i32) {
        eprintln!("Written {count} coincidences to database.");
    }
    fn warn(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }
    fn vpn_rotating(&self) {
        eprintln!("VPN: rotating IP...");
    }
    fn vpn_rotated(&self) {
        eprintln!("VPN: IP rotated successfully");
    }
    fn vpn_not_available(&self) {
        eprintln!("Warning: NordVPN binary not found, continuing without IP rotation");
    }
}
