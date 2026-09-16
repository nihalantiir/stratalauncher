//! System RAM detection, so the default memory allocation adapts to the
//! machine instead of being one fixed number for everyone.

#[cfg(windows)]
pub fn total_memory_mb() -> Option<u32> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return None;
    }
    Some((status.ullTotalPhys / (1024 * 1024)) as u32)
}

#[cfg(not(windows))]
pub fn total_memory_mb() -> Option<u32> {
    None
}

/// Roughly half of `total_mb`, capped at 4GB (modern Minecraft's own
/// recommendation) and floored at 1GB, rounded to the UI's 256MB step.
fn clamp_recommendation(total_mb: u32) -> u32 {
    ((total_mb / 2).clamp(1024, 4096) / 256) * 256
}

/// `clamp_recommendation` of the real detected total; 2GB if detection fails.
pub fn recommended_memory_mb() -> u32 {
    total_memory_mb().map(clamp_recommendation).unwrap_or(2048)
}

#[cfg(test)]
mod tests {
    use super::clamp_recommendation;

    #[test]
    fn scales_with_total_ram_within_sane_bounds() {
        assert_eq!(clamp_recommendation(2048), 1024); // low-RAM floor
        assert_eq!(clamp_recommendation(8192), 4096); // half, at the cap
        assert_eq!(clamp_recommendation(32768), 4096); // huge machine, still capped
        assert_eq!(clamp_recommendation(6000), 2816); // mid-range, rounds to 256MB step
    }
}
