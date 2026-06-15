use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::verboser::Verboser;

const NORDVPN_EXE: &str = "nordvpn.exe";

const VPN_COUNTRIES: &[&str] = &[
    "Spain",
    "France",
    "Germany",
    "Italy",
    "Portugal",
    "switzerland",
    "Belgium",
    "India",
    "Sweden",
    "Ireland",
    "Brazil",
    "Mexico",
    "Albania",
    "Mexico",
    "Poland",
    "United States",
    "South Korea",
    "Hong Kong",
    "Singapore",
    "Austria",
    "Norway",
];

fn random_country() -> &'static str {
    let idx = rand::random::<usize>() % VPN_COUNTRIES.len();
    VPN_COUNTRIES[idx]
}

fn resolve_exe(nordvpn_path: &Path) -> PathBuf {
    nordvpn_path.join(NORDVPN_EXE)
}

pub fn nordvpn_available(nordvpn_path: &Path) -> bool {
    resolve_exe(nordvpn_path).exists()
}

pub async fn rotate_vpn(nordvpn_path: &Path, verboser: &impl Verboser) {
    verboser.vpn_rotating();
    let exe = resolve_exe(nordvpn_path);

    let _ = Command::new(&exe).arg("-d").kill_on_drop(true).status().await;

    let _ = Command::new(&exe)
        .args(["-c", "-g", random_country()])
        .kill_on_drop(true)
        .status()
        .await;

    verboser.vpn_rotated();
}
