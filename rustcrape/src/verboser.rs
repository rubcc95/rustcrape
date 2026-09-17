use std::time::Duration;
use crate::types::Coincidence;

pub trait Verboser: Send + Sync + 'static {
    // Preparacion / base de datos
    fn seeding_tasks(&self, checked: usize, valid: usize, total: usize);
    fn connecting_db(&self);
    fn creating_db(&self);
    fn verifying_db(&self);
    fn creating_tables(&self);

    // Motor
    fn rate_limit_wait(&self, wait: Duration);
    fn obtaining_task(&self);
    fn claimed_task(&self, label: &str);
    fn released_task(&self);
    fn finished(&self);
    fn opening_browser(&self, label: &str);
    fn closing_browser(&self);

    // Scraping
    fn scraping_start(&self);
    fn accepting_cookies(&self);
    fn searching_coincidences(&self);
    fn found_single_coincidence(&self);
    fn found_multiple_coincidences(&self);
    fn processed_coincidence(&self, name: &str, count: usize);
    fn writing_coincidences(&self, output: &[Coincidence]);
    fn written_coincidences(&self, count: i32);

    // VPN
    fn vpn_rotating(&self) {}
    fn vpn_rotated(&self) {}
    fn vpn_not_available(&self) {}

    fn warn(&self, msg: &str);
    fn error(&self, err: &str);
    fn debug(&self, err: &str);

    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Clone, Default)]
pub struct NoVerboser;

impl Verboser for NoVerboser {
    #[inline(always)]
    fn seeding_tasks(&self, _: usize, _: usize, _: usize) {}
    #[inline(always)]
    fn connecting_db(&self) {}
    #[inline(always)]
    fn creating_db(&self) {}
    #[inline(always)]
    fn verifying_db(&self) {}
    #[inline(always)]
    fn creating_tables(&self) {}
    #[inline(always)]
    fn rate_limit_wait(&self, _: Duration) {}
    #[inline(always)]
    fn obtaining_task(&self) {}
    #[inline(always)]
    fn claimed_task(&self, _: &str) {}
    #[inline(always)]
    fn released_task(&self) {}
    #[inline(always)]
    fn finished(&self) {}
    #[inline(always)]
    fn opening_browser(&self, _: &str) {}
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
    fn vpn_rotating(&self) {}
    #[inline(always)]
    fn vpn_rotated(&self) {}
    #[inline(always)]
    fn vpn_not_available(&self) {}
    #[inline(always)]
    fn warn(&self, _: &str) {}
    #[inline(always)]
    fn error(&self, _: &str) {}
    #[inline(always)]
    fn debug(&self, _: &str) {}
}

#[cfg(debug_assertions)]
#[derive(Clone, Default)]
pub struct DebugVerboser;

#[cfg(debug_assertions)]
impl Verboser for DebugVerboser {
    fn seeding_tasks(&self, checked: usize, valid: usize, total: usize) {
        eprintln!("Seeding tasks: checked = {checked}, valid = {valid}, total = {total}",);
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
    fn rate_limit_wait(&self, wait: Duration) {
        let total_secs = wait.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        eprintln!("Rate limit reached. Waiting for {mins}:{secs:02} minutes...");
    }
    fn obtaining_task(&self) {
        eprintln!("Obtaining task to process...");
    }
    fn claimed_task(&self, label: &str) {
        eprintln!("Claimed task at {label}");
    }
    fn released_task(&self) {
        eprintln!("Released task");
    }
    fn finished(&self) {
        eprintln!("Finished processing all tasks.");
    }
    fn opening_browser(&self, label: &str) {
        eprintln!("Opening browser at {label}...");
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
    fn vpn_rotating(&self) {
        eprintln!("VPN: rotating IP...");
    }
    fn vpn_rotated(&self) {
        eprintln!("VPN: IP rotated successfully");
    }
    fn vpn_not_available(&self) {
        eprintln!("Warning: NordVPN binary not found, continuing without IP rotation");
    }
    fn warn(&self, msg: &str) {
        eprintln!("Warning: {msg}");
    }
    fn error(&self, err: &str) {
        eprintln!("Error: {err}");
    }
    fn debug(&self, err: &str) {
        eprintln!("Debug: {err}");
    }
}
