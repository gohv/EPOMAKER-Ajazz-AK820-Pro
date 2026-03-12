/// System stats collection for LCD display.

use sysinfo::System;
use std::path::PathBuf;

pub struct SystemStats {
    sys: System,
    /// Path to the coretemp hwmon temp input file (e.g., /sys/class/hwmon/hwmonN/temp1_input)
    cpu_temp_path: Option<PathBuf>,
    gpu_temp_method: GpuTempMethod,
}

enum GpuTempMethod {
    NvidiaSmi,
    Hwmon(PathBuf),
    None,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_percent: f32,
}

impl SystemStats {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_temp_path = find_coretemp_path();
        let gpu_temp_method = find_gpu_temp_method();

        Self { sys, cpu_temp_path, gpu_temp_method }
    }

    pub fn refresh(&mut self) -> Stats {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let cpu_usage = self.sys.global_cpu_usage();
        let mem_used = self.sys.used_memory() as f64 / 1_073_741_824.0;
        let mem_total = self.sys.total_memory() as f64 / 1_073_741_824.0;
        let mem_pct = if mem_total > 0.0 {
            (mem_used / mem_total * 100.0) as f32
        } else {
            0.0
        };

        let cpu_temp_c = self.cpu_temp_path.as_ref().and_then(|p| {
            std::fs::read_to_string(p)
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|millideg| millideg / 1000.0)
        });

        let gpu_temp_c = match &self.gpu_temp_method {
            GpuTempMethod::NvidiaSmi => read_nvidia_smi_temp(),
            GpuTempMethod::Hwmon(path) => std::fs::read_to_string(path)
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|millideg| millideg / 1000.0),
            GpuTempMethod::None => None,
        };

        Stats {
            cpu_usage,
            cpu_temp_c,
            gpu_temp_c,
            memory_used_gb: mem_used,
            memory_total_gb: mem_total,
            memory_percent: mem_pct,
        }
    }
}

/// Find the coretemp hwmon device and return the path to temp1_input (package temp).
fn find_coretemp_path() -> Option<PathBuf> {
    let hwmon_dir = std::fs::read_dir("/sys/class/hwmon/").ok()?;
    for entry in hwmon_dir.flatten() {
        let name_path = entry.path().join("name");
        if let Ok(name) = std::fs::read_to_string(&name_path) {
            if name.trim() == "coretemp" {
                let temp_path = entry.path().join("temp1_input");
                if temp_path.exists() {
                    eprintln!("Found CPU temp sensor: {}", temp_path.display());
                    return Some(temp_path);
                }
            }
        }
    }
    // Fallback: try k10temp (AMD) or other common names
    for fallback_name in &["k10temp", "zenpower", "acpitz"] {
        let hwmon_dir = std::fs::read_dir("/sys/class/hwmon/").ok()?;
        for entry in hwmon_dir.flatten() {
            let name_path = entry.path().join("name");
            if let Ok(name) = std::fs::read_to_string(&name_path) {
                if name.trim() == *fallback_name {
                    let temp_path = entry.path().join("temp1_input");
                    if temp_path.exists() {
                        eprintln!("Found CPU temp sensor ({}): {}", fallback_name, temp_path.display());
                        return Some(temp_path);
                    }
                }
            }
        }
    }
    eprintln!("Warning: no CPU temperature sensor found");
    None
}

/// Find the best method for reading GPU temperature.
fn find_gpu_temp_method() -> GpuTempMethod {
    // Check for nvidia-smi first (works for NVIDIA GPUs)
    if std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu")
        .arg("--format=csv,noheader")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        eprintln!("Found GPU temp sensor: nvidia-smi");
        return GpuTempMethod::NvidiaSmi;
    }
    // Check hwmon for amdgpu, i915
    if let Ok(hwmon_dir) = std::fs::read_dir("/sys/class/hwmon/") {
        for entry in hwmon_dir.flatten() {
            let name_path = entry.path().join("name");
            if let Ok(name) = std::fs::read_to_string(&name_path) {
                let name = name.trim();
                if name == "amdgpu" || name == "i915" || name == "nouveau" {
                    let temp_path = entry.path().join("temp1_input");
                    if temp_path.exists() {
                        eprintln!("Found GPU temp sensor ({}): {}", name, temp_path.display());
                        return GpuTempMethod::Hwmon(temp_path);
                    }
                }
            }
        }
    }
    eprintln!("Warning: no GPU temperature sensor found");
    GpuTempMethod::None
}

fn read_nvidia_smi_temp() -> Option<f32> {
    std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=temperature.gpu")
        .arg("--format=csv,noheader")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f32>().ok())
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CPU: {:.0}%", self.cpu_usage)?;
        if let Some(temp) = self.cpu_temp_c {
            write!(f, " {:.0}°C", temp)?;
        }
        if let Some(temp) = self.gpu_temp_c {
            write!(f, " | GPU: {:.0}°C", temp)?;
        }
        write!(
            f, " | RAM: {:.1}/{:.1} GB ({:.0}%)",
            self.memory_used_gb, self.memory_total_gb, self.memory_percent
        )
    }
}
