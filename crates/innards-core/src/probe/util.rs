use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Read a sysfs/procfs-style file and trim it. `None` on any error.
pub fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn read_u64(path: impl AsRef<Path>) -> Option<u64> {
    read_trim(path)?.parse().ok()
}

/// Run a command with a timeout-ish guard and return stdout if it exited 0.
/// We don't have a real timeout without extra deps; probes only call tools
/// that return promptly (smartctl, system_profiler, powershell).
pub fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

pub fn which(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    let exts: Vec<String> = if cfg!(windows) {
        vec!["".into(), ".exe".into(), ".cmd".into()]
    } else {
        vec!["".into()]
    };
    std::env::split_paths(&path).any(|dir| exts.iter().any(|e| dir.join(format!("{cmd}{e}")).is_file()))
}

/// Infer an approximate launch year from a CPU brand string. Coarse by
/// design; only used to say "this is an N-year-old CPU".
pub fn cpu_launch_year(brand: &str) -> Option<u16> {
    let b = brand.to_ascii_lowercase();
    // Apple silicon
    for (needle, year) in [("m1", 2020), ("m2", 2022), ("m3", 2023), ("m4", 2024), ("m5", 2025)] {
        if b.contains(&format!("apple {needle}")) {
            return Some(year);
        }
    }
    // Intel Core iN-GGGG: generation from the leading digits of the model number.
    if let Some(pos) = b.find("core(tm) i").or_else(|| b.find("core i")) {
        let rest = &b[pos..];
        if let Some(dash) = rest.find('-') {
            let digits: String = rest[dash + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            let gen = match digits.len() {
                4 => digits[..1].parse::<u16>().ok(),
                5 => digits[..2].parse::<u16>().ok(),
                _ => None,
            }?;
            return Some(match gen {
                1 => 2010, 2 => 2011, 3 => 2012, 4 => 2013, 5 => 2015, 6 => 2015, 7 => 2017,
                8 => 2018, 9 => 2019, 10 => 2020, 11 => 2021, 12 => 2022, 13 => 2023,
                14 => 2024, _ => 2025,
            });
        }
    }
    if b.contains("core ultra") {
        return Some(if b.contains("series 2") || b.contains("2xx") { 2025 } else { 2024 });
    }
    // AMD Ryzen N GGGG
    if let Some(pos) = b.find("ryzen") {
        let digits: String = b[pos..].split_whitespace().nth(2).unwrap_or("").chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() == 4 {
            return match &digits[..1] {
                "1" => Some(2017), "2" => Some(2018), "3" => Some(2019), "4" => Some(2020),
                "5" => Some(2021), "6" => Some(2022), "7" => Some(2023), "8" => Some(2024),
                "9" => Some(2024), _ => None,
            };
        }
        if b.contains("ai 3") {
            return Some(2024);
        }
    }
    None
}

/// Split "nvme0n1p5" -> "nvme0n1", "sda2" -> "sda", "mmcblk0p1" -> "mmcblk0".
pub fn parent_block_device(part: &str) -> String {
    // "/dev/sda2", "/dev/block/sda13" (Android) -> "sda2" / "sda13".
    let p = part.rsplit('/').next().unwrap_or(part);
    if p.starts_with("nvme") || p.starts_with("mmcblk") {
        if let Some(idx) = p.rfind('p') {
            if p[idx + 1..].chars().all(|c| c.is_ascii_digit()) && !p[idx + 1..].is_empty() {
                return p[..idx].to_string();
            }
        }
        return p.to_string();
    }
    p.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

pub fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}
