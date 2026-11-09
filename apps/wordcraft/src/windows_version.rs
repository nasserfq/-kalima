//! Shared by `apps/wordcraft/build.rs` and `apps/wordcraft-cli/build.rs`: the Windows file details
//! (Explorer > Properties > Details) for كلمة's executables. They name كلمة and its maker, credit
//! WordCraft, and carry the release version (`KALIMA_VERSION`, set by the release workflow), not
//! the WordCraft crate version.

/// Product name shown in the Details tab, the taskbar and Task Manager.
pub const PRODUCT_NAME: &str = "كلمة (Kalima)";
pub const COMPANY_NAME: &str = "Nasser Alqahtani";
pub const COPYRIGHT: &str = "Copyright (c) Nasser Alqahtani. Based on WordCraft, copyright (c) the WordCraft authors. MIT OR Apache-2.0.";

/// The version the executables report: `KALIMA_VERSION` when set, else the crate version.
pub fn version(package: &str) -> String {
    std::env::var("KALIMA_VERSION").ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| package.to_string())
}

/// `MAJOR << 48 | MINOR << 32 | PATCH << 16`, the numeric form of `version`.
pub fn version_number(version: &str) -> u64 {
    let mut parts = version.split(['.', '-', '+']).map(|p| p.parse::<u16>().unwrap_or(0));
    let mut number = 0u64;
    for shift in [48, 32, 16] {
        number |= u64::from(parts.next().unwrap_or(0)) << shift;
    }
    number
}
