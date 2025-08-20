//! # Hardware Detection and Optimization
//!
//! This module provides utilities for detecting system hardware capabilities
//! and calculating optimal model parameters for the best performance without
//! overwhelming the system.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use thiserror::Error;

/// Hardware detection errors
#[derive(Debug, Error)]
pub enum HardwareError {
    /// Failed to detect system memory
    #[error("Failed to detect system memory: {0}")]
    MemoryDetectionFailed(String),

    /// Failed to detect CPU information
    #[error("Failed to detect CPU information: {0}")]
    CpuDetectionFailed(String),

    /// Failed to detect GPU information
    #[error("Failed to detect GPU information: {0}")]
    GpuDetectionFailed(String),

    /// Unsupported platform
    #[error("Hardware detection not supported on this platform")]
    UnsupportedPlatform,
}

/// GPU type detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuType {
    /// NVIDIA GPU with CUDA support
    Nvidia,
    /// AMD GPU
    Amd,
    /// Apple Silicon GPU (Metal)
    AppleSilicon,
    /// Intel integrated graphics
    Intel,
    /// No GPU or unsupported GPU
    None,
}

/// System hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHardware {
    /// Total system memory in bytes
    pub total_memory: u64,
    /// Available system memory in bytes
    pub available_memory: u64,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Number of logical CPU threads
    pub cpu_threads: u32,
    /// CPU brand/model information
    pub cpu_model: String,
    /// GPU type and capabilities
    pub gpu_type: GpuType,
    /// GPU memory in bytes (if available)
    pub gpu_memory: Option<u64>,
    /// Whether the system supports memory mapping
    pub supports_mmap: bool,
    /// Architecture (x86_64, aarch64, etc.)
    pub architecture: String,
}

/// Optimal model loading parameters based on hardware
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalParams {
    /// Recommended number of CPU threads
    pub n_threads: u32,
    /// Recommended number of GPU layers
    pub n_gpu_layers: u32,
    /// Recommended context size
    pub context_size: u32,
    /// Recommended batch size
    pub batch_size: u32,
    /// Maximum recommended model size in bytes
    pub max_model_size: u64,
    /// Whether to use memory mapping
    pub use_mmap: bool,
    /// Memory buffer percentage to reserve for system
    pub memory_buffer_percent: f32,
}

/// Global hardware information cache
static HARDWARE_INFO: OnceLock<SystemHardware> = OnceLock::new();

impl SystemHardware {
    /// Detect system hardware capabilities
    ///
    /// This function performs a comprehensive scan of the system's hardware
    /// capabilities and caches the result for subsequent calls.
    ///
    /// # Returns
    ///
    /// A [`SystemHardware`] struct containing detected hardware information.
    ///
    /// # Errors
    ///
    /// Returns a [`HardwareError`] if hardware detection fails on any component.
    pub fn detect() -> Result<Self, HardwareError> {
        // Check cache first
        if let Some(cached) = HARDWARE_INFO.get() {
            return Ok(cached.clone());
        }

        let hardware = Self::detect_fresh()?;

        // Cache the result
        let _ = HARDWARE_INFO.set(hardware.clone());

        Ok(hardware)
    }

    /// Perform fresh hardware detection without using cache
    fn detect_fresh() -> Result<Self, HardwareError> {
        let (total_memory, available_memory) = Self::detect_memory()?;
        let (cpu_cores, cpu_threads, cpu_model) = Self::detect_cpu()?;
        let (gpu_type, gpu_memory) = Self::detect_gpu()?;
        let supports_mmap = Self::detect_mmap_support();
        let architecture = Self::detect_architecture();

        Ok(SystemHardware {
            total_memory,
            available_memory,
            cpu_cores,
            cpu_threads,
            cpu_model,
            gpu_type,
            gpu_memory,
            supports_mmap,
            architecture,
        })
    }

    /// Detect system memory information
    fn detect_memory() -> Result<(u64, u64), HardwareError> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let meminfo = fs::read_to_string("/proc/meminfo")
                .map_err(|e| HardwareError::MemoryDetectionFailed(e.to_string()))?;

            let mut total_kb = 0u64;
            let mut available_kb = 0u64;

            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
            }

            if total_kb == 0 {
                return Err(HardwareError::MemoryDetectionFailed(
                    "Could not parse MemTotal from /proc/meminfo".to_string(),
                ));
            }

            Ok((total_kb * 1024, available_kb * 1024))
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // Get total memory
            let total_output = Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .map_err(|e| HardwareError::MemoryDetectionFailed(e.to_string()))?;

            let total_memory: u64 = String::from_utf8_lossy(&total_output.stdout)
                .trim()
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    HardwareError::MemoryDetectionFailed(e.to_string())
                })?;

            // Get available memory (approximation using vm_stat)
            let vm_output = Command::new("vm_stat")
                .output()
                .map_err(|e| HardwareError::MemoryDetectionFailed(e.to_string()))?;

            let vm_stat = String::from_utf8_lossy(&vm_output.stdout);
            let page_size = 4096u64; // Default page size on macOS

            let mut free_pages = 0u64;
            let mut inactive_pages = 0u64;

            for line in vm_stat.lines() {
                if line.contains("Pages free:") {
                    free_pages = line
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.trim_end_matches('.').parse().ok())
                        .unwrap_or(0);
                } else if line.contains("Pages inactive:") {
                    inactive_pages = line
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.trim_end_matches('.').parse().ok())
                        .unwrap_or(0);
                }
            }

            let available_memory = (free_pages + inactive_pages) * page_size;

            Ok((total_memory, available_memory))
        }

        #[cfg(target_os = "windows")]
        {
            // For Windows, we'll provide a conservative estimate
            // In a real implementation, you'd use Windows API calls
            Err(HardwareError::UnsupportedPlatform)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(HardwareError::UnsupportedPlatform)
        }
    }

    /// Detect CPU information
    fn detect_cpu() -> Result<(u32, u32, String), HardwareError> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;

            let cpuinfo = fs::read_to_string("/proc/cpuinfo")
                .map_err(|e| HardwareError::CpuDetectionFailed(e.to_string()))?;

            let mut cpu_cores = 0u32;
            let mut cpu_model = String::from("Unknown CPU");

            for line in cpuinfo.lines() {
                if line.starts_with("processor") {
                    cpu_cores += 1;
                } else if line.starts_with("model name") && cpu_model == "Unknown CPU" {
                    if let Some(model) = line.split(':').nth(1) {
                        cpu_model = model.trim().to_string();
                    }
                }
            }

            // Logical threads same as cores for now
            let cpu_threads = cpu_cores;

            Ok((cpu_cores, cpu_threads, cpu_model))
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // Get CPU cores
            let cores_output = Command::new("sysctl")
                .args(["-n", "hw.physicalcpu"])
                .output()
                .map_err(|e| HardwareError::CpuDetectionFailed(e.to_string()))?;

            let cpu_cores: u32 = String::from_utf8_lossy(&cores_output.stdout)
                .trim()
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    HardwareError::CpuDetectionFailed(e.to_string())
                })?;

            // Get logical CPU count
            let threads_output = Command::new("sysctl")
                .args(["-n", "hw.logicalcpu"])
                .output()
                .map_err(|e| HardwareError::CpuDetectionFailed(e.to_string()))?;

            let cpu_threads: u32 = String::from_utf8_lossy(&threads_output.stdout)
                .trim()
                .parse()
                .map_err(|e: std::num::ParseIntError| {
                    HardwareError::CpuDetectionFailed(e.to_string())
                })?;

            // Get CPU model
            let model_output = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .map_err(|e| HardwareError::CpuDetectionFailed(e.to_string()))?;

            let cpu_model = String::from_utf8_lossy(&model_output.stdout)
                .trim()
                .to_string();

            Ok((cpu_cores, cpu_threads, cpu_model))
        }

        #[cfg(target_os = "windows")]
        {
            Err(HardwareError::UnsupportedPlatform)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(HardwareError::UnsupportedPlatform)
        }
    }

    /// Detect GPU information
    fn detect_gpu() -> Result<(GpuType, Option<u64>), HardwareError> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // Check for Apple Silicon
            let arch_output = Command::new("uname")
                .arg("-m")
                .output()
                .map_err(|e| HardwareError::GpuDetectionFailed(e.to_string()))?;

            let arch = String::from_utf8_lossy(&arch_output.stdout);

            if arch.contains("arm64") {
                // Apple Silicon has integrated GPU
                return Ok((GpuType::AppleSilicon, None));
            }

            // For Intel Macs, check system_profiler
            let gpu_output = Command::new("system_profiler")
                .arg("SPDisplaysDataType")
                .output()
                .map_err(|e| HardwareError::GpuDetectionFailed(e.to_string()))?;

            let gpu_info = String::from_utf8_lossy(&gpu_output.stdout);

            if gpu_info.to_lowercase().contains("nvidia") {
                Ok((GpuType::Nvidia, None))
            } else if gpu_info.to_lowercase().contains("amd")
                || gpu_info.to_lowercase().contains("radeon")
            {
                Ok((GpuType::Amd, None))
            } else if gpu_info.to_lowercase().contains("intel") {
                Ok((GpuType::Intel, None))
            } else {
                Ok((GpuType::None, None))
            }
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Command;

            // Check for NVIDIA GPU
            if let Ok(output) = Command::new("nvidia-smi")
                .arg("--query-gpu=memory.total")
                .arg("--format=csv,noheader,nounits")
                .output()
            {
                if output.status.success() {
                    let memory_str = String::from_utf8_lossy(&output.stdout);
                    if let Ok(memory_mb) = memory_str.trim().parse::<u64>() {
                        return Ok((GpuType::Nvidia, Some(memory_mb * 1024 * 1024)));
                    }
                    return Ok((GpuType::Nvidia, None));
                }
            }

            // Check for AMD GPU
            if let Ok(output) = Command::new("lspci").output() {
                let lspci_output = String::from_utf8_lossy(&output.stdout);
                if lspci_output.to_lowercase().contains("amd")
                    || lspci_output.to_lowercase().contains("radeon")
                {
                    return Ok((GpuType::Amd, None));
                }
                if lspci_output.to_lowercase().contains("intel")
                    && lspci_output.to_lowercase().contains("graphics")
                {
                    return Ok((GpuType::Intel, None));
                }
            }

            Ok((GpuType::None, None))
        }

        #[cfg(target_os = "windows")]
        {
            Err(HardwareError::UnsupportedPlatform)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(HardwareError::UnsupportedPlatform)
        }
    }

    /// Detect memory mapping support
    fn detect_mmap_support() -> bool {
        // Most Unix-like systems support mmap
        #[cfg(unix)]
        {
            true
        }

        #[cfg(windows)]
        {
            true // Windows supports memory mapping
        }

        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }

    /// Detect system architecture
    fn detect_architecture() -> String {
        std::env::consts::ARCH.to_string()
    }

    /// Calculate optimal parameters for model loading
    ///
    /// This method analyzes the detected hardware and calculates the best
    /// parameters for model loading that maximize performance while ensuring
    /// system stability.
    ///
    /// # Arguments
    ///
    /// * `model_size_estimate` - Estimated model size in bytes (optional)
    ///
    /// # Returns
    ///
    /// [`OptimalParams`] with recommended settings for optimal performance.
    pub fn calculate_optimal_params(&self, model_size_estimate: Option<u64>) -> OptimalParams {
        // Calculate safe memory usage (leave buffer for system)
        let memory_buffer_percent = match self.total_memory {
            mem if mem < 4 * 1024 * 1024 * 1024 => 0.3, // 30% buffer for systems with < 4GB
            mem if mem < 8 * 1024 * 1024 * 1024 => 0.25, // 25% buffer for systems with < 8GB
            mem if mem < 16 * 1024 * 1024 * 1024 => 0.2, // 20% buffer for systems with < 16GB
            _ => 0.15,                                  // 15% buffer for systems with >= 16GB
        };

        let available_for_model =
            (self.available_memory as f32 * (1.0 - memory_buffer_percent)) as u64;

        // Calculate optimal thread count
        let n_threads = match self.cpu_cores {
            cores if cores <= 2 => cores.max(1),
            cores if cores <= 4 => cores - 1, // Leave one core for system
            cores if cores <= 8 => cores - 2, // Leave two cores for system
            cores => cores.min(12),           // Cap at 12 for diminishing returns
        };

        // Calculate GPU layers based on GPU type and memory
        let n_gpu_layers = match &self.gpu_type {
            GpuType::None | GpuType::Intel => 0, // No GPU acceleration
            GpuType::AppleSilicon => {
                // Apple Silicon can use significant GPU acceleration
                match self.total_memory {
                    mem if mem >= 16 * 1024 * 1024 * 1024 => 35, // 16GB+ unified memory
                    mem if mem >= 8 * 1024 * 1024 * 1024 => 25,  // 8GB+ unified memory
                    _ => 15,                                     // Lower memory systems
                }
            }
            GpuType::Nvidia => {
                if let Some(gpu_mem) = self.gpu_memory {
                    match gpu_mem {
                        mem if mem >= 12 * 1024 * 1024 * 1024 => 35, // 12GB+ VRAM
                        mem if mem >= 8 * 1024 * 1024 * 1024 => 28,  // 8GB+ VRAM
                        mem if mem >= 6 * 1024 * 1024 * 1024 => 20,  // 6GB+ VRAM
                        mem if mem >= 4 * 1024 * 1024 * 1024 => 15,  // 4GB+ VRAM
                        _ => 10,                                     // Lower VRAM
                    }
                } else {
                    // Conservative estimate without memory info
                    15
                }
            }
            GpuType::Amd => {
                // AMD GPU support varies, be more conservative
                if let Some(gpu_mem) = self.gpu_memory {
                    match gpu_mem {
                        mem if mem >= 8 * 1024 * 1024 * 1024 => 20,
                        mem if mem >= 4 * 1024 * 1024 * 1024 => 10,
                        _ => 5,
                    }
                } else {
                    10
                }
            }
        };

        // Calculate context size based on available memory
        let context_size = if let Some(model_size) = model_size_estimate {
            // Leave enough memory for model + context + overhead
            let remaining_memory = available_for_model.saturating_sub(model_size);
            match remaining_memory {
                mem if mem >= 4 * 1024 * 1024 * 1024 => 8192, // 4GB+ remaining
                mem if mem >= 2 * 1024 * 1024 * 1024 => 4096, // 2GB+ remaining
                mem if mem >= 1024 * 1024 * 1024 => 2048,     // 1GB+ remaining
                _ => 1024,                                    // Conservative for low memory
            }
        } else {
            // Default context sizes based on total memory
            match self.total_memory {
                mem if mem >= 32 * 1024 * 1024 * 1024 => 8192, // 32GB+
                mem if mem >= 16 * 1024 * 1024 * 1024 => 4096, // 16GB+
                mem if mem >= 8 * 1024 * 1024 * 1024 => 2048,  // 8GB+
                _ => 1024,                                     // Conservative default
            }
        };

        // Calculate batch size
        let batch_size = match self.total_memory {
            mem if mem >= 16 * 1024 * 1024 * 1024 => 1024, // 16GB+
            mem if mem >= 8 * 1024 * 1024 * 1024 => 512,   // 8GB+
            _ => 256,                                      // Conservative for lower memory
        };

        // Maximum model size we can safely load
        let max_model_size = (available_for_model as f32 * 0.7) as u64; // Use 70% of available memory

        OptimalParams {
            n_threads,
            n_gpu_layers,
            context_size,
            batch_size,
            max_model_size,
            use_mmap: self.supports_mmap,
            memory_buffer_percent,
        }
    }

    /// Get a human-readable description of the detected hardware
    ///
    /// Returns a formatted string containing CPU model, core count, memory information,
    /// and GPU details for debugging and informational purposes.
    ///
    /// # Returns
    ///
    /// A [`String`](https://doc.rust-lang.org/std/string/struct.String.html) containing
    /// a comprehensive hardware description.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use shared::llms::SystemHardware;
    ///
    /// let hardware = SystemHardware::detect()?;
    /// println!("System: {}", hardware.description());
    /// ```
    pub fn description(&self) -> String {
        format!(
            "CPU: {} ({} cores, {} threads), Memory: {:.1}GB total / {:.1}GB available, GPU: {:?}{}",
            self.cpu_model,
            self.cpu_cores,
            self.cpu_threads,
            self.total_memory as f64 / (1024.0 * 1024.0 * 1024.0),
            self.available_memory as f64 / (1024.0 * 1024.0 * 1024.0),
            self.gpu_type,
            if let Some(gpu_mem) = self.gpu_memory {
                format!(
                    " ({:.1}GB VRAM)",
                    gpu_mem as f64 / (1024.0 * 1024.0 * 1024.0)
                )
            } else {
                String::new()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_type_serialization() {
        let gpu_types = vec![
            GpuType::Nvidia,
            GpuType::Amd,
            GpuType::AppleSilicon,
            GpuType::Intel,
            GpuType::None,
        ];

        for gpu_type in gpu_types {
            let serialized = serde_json::to_string(&gpu_type).unwrap();
            let deserialized: GpuType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(gpu_type, deserialized);
        }
    }

    #[test]
    fn test_optimal_params_calculation() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,     // 16GB
            available_memory: 12 * 1024 * 1024 * 1024, // 12GB available
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Test CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(8 * 1024 * 1024 * 1024), // 8GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(Some(4 * 1024 * 1024 * 1024)); // 4GB model

        assert!(params.n_threads > 0);
        assert!(params.n_threads <= hardware.cpu_cores);
        assert!(params.n_gpu_layers > 0); // Should use GPU
        assert!(params.context_size >= 1024);
        assert!(params.batch_size > 0);
        assert!(params.use_mmap);
        assert!(params.memory_buffer_percent > 0.0 && params.memory_buffer_percent < 1.0);
    }

    #[test]
    fn test_low_memory_system() {
        let hardware = SystemHardware {
            total_memory: 4 * 1024 * 1024 * 1024,     // 4GB
            available_memory: 2 * 1024 * 1024 * 1024, // 2GB available
            cpu_cores: 2,
            cpu_threads: 4,
            cpu_model: "Low-end CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 0); // No GPU
        assert!(params.context_size <= 2048); // Conservative context size
        assert!(params.memory_buffer_percent >= 0.25); // Higher buffer for low memory
    }

    #[test]
    fn test_apple_silicon_optimization() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024, // 16GB unified memory
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 8,
            cpu_model: "Apple M1 Pro".to_string(),
            gpu_type: GpuType::AppleSilicon,
            gpu_memory: None, // Unified memory
            supports_mmap: true,
            architecture: "aarch64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert!(params.n_gpu_layers > 20); // Should use significant GPU acceleration
        assert!(params.use_mmap);
    }

    #[test]
    fn test_hardware_error_display() {
        let memory_error = HardwareError::MemoryDetectionFailed("test error".to_string());
        assert_eq!(
            memory_error.to_string(),
            "Failed to detect system memory: test error"
        );

        let cpu_error = HardwareError::CpuDetectionFailed("cpu error".to_string());
        assert_eq!(
            cpu_error.to_string(),
            "Failed to detect CPU information: cpu error"
        );

        let gpu_error = HardwareError::GpuDetectionFailed("gpu error".to_string());
        assert_eq!(
            gpu_error.to_string(),
            "Failed to detect GPU information: gpu error"
        );

        let platform_error = HardwareError::UnsupportedPlatform;
        assert_eq!(
            platform_error.to_string(),
            "Hardware detection not supported on this platform"
        );
    }

    #[test]
    fn test_hardware_error_debug() {
        let error = HardwareError::MemoryDetectionFailed("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MemoryDetectionFailed"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_hardware_description() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,     // 16GB
            available_memory: 12 * 1024 * 1024 * 1024, // 12GB
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Intel Core i7".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(8 * 1024 * 1024 * 1024), // 8GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let description = hardware.description();
        assert!(description.contains("Intel Core i7"));
        assert!(description.contains("8 cores, 16 threads"));
        assert!(description.contains("16.0GB total"));
        assert!(description.contains("12.0GB available"));
        assert!(description.contains("Nvidia"));
        assert!(description.contains("8.0GB VRAM"));
    }

    #[test]
    fn test_hardware_description_without_gpu_memory() {
        let hardware = SystemHardware {
            total_memory: 8 * 1024 * 1024 * 1024,     // 8GB
            available_memory: 6 * 1024 * 1024 * 1024, // 6GB
            cpu_cores: 4,
            cpu_threads: 4,
            cpu_model: "Apple M1".to_string(),
            gpu_type: GpuType::AppleSilicon,
            gpu_memory: None, // Unified memory
            supports_mmap: true,
            architecture: "aarch64".to_string(),
        };

        let description = hardware.description();
        assert!(description.contains("Apple M1"));
        assert!(description.contains("4 cores, 4 threads"));
        assert!(description.contains("8.0GB total"));
        assert!(description.contains("6.0GB available"));
        assert!(description.contains("AppleSilicon"));
        assert!(!description.contains("VRAM")); // Should not contain VRAM info
    }

    #[test]
    fn test_detect_mmap_support() {
        let supports_mmap = SystemHardware::detect_mmap_support();
        // Should be true on Unix-like systems (macOS, Linux)
        assert!(supports_mmap);
    }

    #[test]
    fn test_detect_architecture() {
        let architecture = SystemHardware::detect_architecture();
        // Should return the current architecture
        assert!(!architecture.is_empty());
        assert!(
            architecture == "x86_64" || architecture == "aarch64" || architecture.contains("x86")
        );
    }

    #[test]
    fn test_gpu_type_debug() {
        let gpu_type = GpuType::Nvidia;
        let debug_str = format!("{:?}", gpu_type);
        assert_eq!(debug_str, "Nvidia");
    }

    #[test]
    fn test_system_hardware_serialization() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Test CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(8 * 1024 * 1024 * 1024),
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let serialized = serde_json::to_string(&hardware).unwrap();
        let deserialized: SystemHardware = serde_json::from_str(&serialized).unwrap();

        assert_eq!(hardware.total_memory, deserialized.total_memory);
        assert_eq!(hardware.available_memory, deserialized.available_memory);
        assert_eq!(hardware.cpu_cores, deserialized.cpu_cores);
        assert_eq!(hardware.cpu_threads, deserialized.cpu_threads);
        assert_eq!(hardware.cpu_model, deserialized.cpu_model);
        assert_eq!(hardware.gpu_type, deserialized.gpu_type);
        assert_eq!(hardware.gpu_memory, deserialized.gpu_memory);
        assert_eq!(hardware.supports_mmap, deserialized.supports_mmap);
        assert_eq!(hardware.architecture, deserialized.architecture);
    }

    #[test]
    fn test_system_hardware_debug() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Test CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(8 * 1024 * 1024 * 1024),
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let debug_str = format!("{:?}", hardware);
        assert!(debug_str.contains("SystemHardware"));
        assert!(debug_str.contains("Test CPU"));
        assert!(debug_str.contains("Nvidia"));
    }

    #[test]
    fn test_optimal_params_serialization() {
        let params = OptimalParams {
            n_threads: 8,
            n_gpu_layers: 20,
            context_size: 2048,
            batch_size: 512,
            max_model_size: 8 * 1024 * 1024 * 1024,
            use_mmap: true,
            memory_buffer_percent: 0.2,
        };

        let serialized = serde_json::to_string(&params).unwrap();
        let deserialized: OptimalParams = serde_json::from_str(&serialized).unwrap();

        assert_eq!(params.n_threads, deserialized.n_threads);
        assert_eq!(params.n_gpu_layers, deserialized.n_gpu_layers);
        assert_eq!(params.context_size, deserialized.context_size);
        assert_eq!(params.batch_size, deserialized.batch_size);
        assert_eq!(params.max_model_size, deserialized.max_model_size);
        assert_eq!(params.use_mmap, deserialized.use_mmap);
        assert_eq!(
            params.memory_buffer_percent,
            deserialized.memory_buffer_percent
        );
    }

    #[test]
    fn test_optimal_params_debug() {
        let params = OptimalParams {
            n_threads: 8,
            n_gpu_layers: 20,
            context_size: 2048,
            batch_size: 512,
            max_model_size: 8 * 1024 * 1024 * 1024,
            use_mmap: true,
            memory_buffer_percent: 0.2,
        };

        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("OptimalParams"));
        assert!(debug_str.contains("n_threads: 8"));
        assert!(debug_str.contains("n_gpu_layers: 20"));
    }

    #[test]
    fn test_high_memory_system_optimization() {
        let hardware = SystemHardware {
            total_memory: 64 * 1024 * 1024 * 1024,     // 64GB
            available_memory: 56 * 1024 * 1024 * 1024, // 56GB available
            cpu_cores: 16,
            cpu_threads: 32,
            cpu_model: "High-end CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(24 * 1024 * 1024 * 1024), // 24GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert!(params.n_threads <= 12); // Should cap at 12 for diminishing returns
        assert!(params.n_gpu_layers == 35); // Should use maximum GPU acceleration
        assert_eq!(params.context_size, 8192); // Should allow large context
        assert_eq!(params.batch_size, 1024); // Large batch size for high memory
        assert_eq!(params.memory_buffer_percent, 0.15); // 15% buffer for high memory systems
    }

    #[test]
    fn test_medium_memory_gpu_optimization() {
        let hardware = SystemHardware {
            total_memory: 8 * 1024 * 1024 * 1024,     // 8GB
            available_memory: 6 * 1024 * 1024 * 1024, // 6GB available
            cpu_cores: 6,
            cpu_threads: 12,
            cpu_model: "Mid-range CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(6 * 1024 * 1024 * 1024), // 6GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(Some(2 * 1024 * 1024 * 1024)); // 2GB model

        assert_eq!(params.n_threads, 4); // 6 cores - 2 for system
        assert_eq!(params.n_gpu_layers, 20); // 6GB VRAM should get 20 layers
        // 8GB total gets 20% buffer, Available for model: 6GB * 0.8 = 4.8GB, remaining: 4.8GB - 2GB = 2.8GB, so 4096 context
        assert_eq!(params.context_size, 4096);
        assert_eq!(params.memory_buffer_percent, 0.2); // 20% buffer for 8GB systems (not < 8GB)
    }

    #[test]
    fn test_amd_gpu_optimization() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "AMD CPU".to_string(),
            gpu_type: GpuType::Amd,
            gpu_memory: Some(8 * 1024 * 1024 * 1024), // 8GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 20); // AMD with 8GB should get 20 layers
        assert!(params.use_mmap);
    }

    #[test]
    fn test_amd_gpu_low_memory() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "AMD CPU".to_string(),
            gpu_type: GpuType::Amd,
            gpu_memory: Some(2 * 1024 * 1024 * 1024), // 2GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 5); // AMD with 2GB should get 5 layers
    }

    #[test]
    fn test_amd_gpu_without_memory_info() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "AMD CPU".to_string(),
            gpu_type: GpuType::Amd,
            gpu_memory: None, // No memory info
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 10); // Conservative default for AMD without memory info
    }

    #[test]
    fn test_intel_gpu_no_acceleration() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Intel CPU".to_string(),
            gpu_type: GpuType::Intel,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 0); // Intel GPU should not use acceleration
    }

    #[test]
    fn test_nvidia_high_vram() {
        let hardware = SystemHardware {
            total_memory: 32 * 1024 * 1024 * 1024,
            available_memory: 28 * 1024 * 1024 * 1024,
            cpu_cores: 12,
            cpu_threads: 24,
            cpu_model: "High-end CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(24 * 1024 * 1024 * 1024), // 24GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 35); // 24GB+ VRAM should get 35 layers
    }

    #[test]
    fn test_nvidia_medium_vram() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(4 * 1024 * 1024 * 1024), // 4GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 15); // 4GB VRAM should get 15 layers
    }

    #[test]
    fn test_nvidia_low_vram() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(2 * 1024 * 1024 * 1024), // 2GB VRAM
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 10); // Low VRAM should get 10 layers
    }

    #[test]
    fn test_nvidia_without_memory_info() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: None, // No memory info
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 15); // Conservative estimate without memory info
    }

    #[test]
    fn test_apple_silicon_8gb() {
        let hardware = SystemHardware {
            total_memory: 8 * 1024 * 1024 * 1024, // 8GB unified memory
            available_memory: 6 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 8,
            cpu_model: "Apple M1".to_string(),
            gpu_type: GpuType::AppleSilicon,
            gpu_memory: None, // Unified memory
            supports_mmap: true,
            architecture: "aarch64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 25); // 8GB+ unified memory should get 25 layers
    }

    #[test]
    fn test_apple_silicon_low_memory() {
        let hardware = SystemHardware {
            total_memory: 4 * 1024 * 1024 * 1024, // 4GB unified memory
            available_memory: 3 * 1024 * 1024 * 1024,
            cpu_cores: 4,
            cpu_threads: 4,
            cpu_model: "Apple M1".to_string(),
            gpu_type: GpuType::AppleSilicon,
            gpu_memory: None, // Unified memory
            supports_mmap: true,
            architecture: "aarch64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_gpu_layers, 15); // Lower memory systems should get 15 layers
    }

    #[test]
    fn test_very_low_cpu_cores() {
        let hardware = SystemHardware {
            total_memory: 4 * 1024 * 1024 * 1024,
            available_memory: 3 * 1024 * 1024 * 1024,
            cpu_cores: 1, // Single core
            cpu_threads: 1,
            cpu_model: "Single Core CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_threads, 1); // Should get at least 1 thread
    }

    #[test]
    fn test_high_cpu_cores() {
        let hardware = SystemHardware {
            total_memory: 64 * 1024 * 1024 * 1024,
            available_memory: 56 * 1024 * 1024 * 1024,
            cpu_cores: 32, // Many cores
            cpu_threads: 64,
            cpu_model: "Many Core CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = hardware.calculate_optimal_params(None);

        assert_eq!(params.n_threads, 12); // Should cap at 12 for diminishing returns
    }

    #[test]
    fn test_context_size_with_large_model() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let large_model_size = 8 * 1024 * 1024 * 1024; // 8GB model
        let params = hardware.calculate_optimal_params(Some(large_model_size));

        // 16GB total gets 15% buffer (>=16GB), Available for model: 12GB * 0.85 = 10.2GB, remaining: 10.2GB - 8GB = 2.2GB, so 4096 context
        assert_eq!(params.context_size, 4096);
    }

    #[test]
    fn test_memory_buffer_percentages() {
        // Test 4GB system (30% buffer)
        let low_memory_hardware = SystemHardware {
            total_memory: 2 * 1024 * 1024 * 1024, // 2GB (less than 4GB)
            available_memory: 1536 * 1024 * 1024, // 1.5GB
            cpu_cores: 2,
            cpu_threads: 4,
            cpu_model: "Low CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = low_memory_hardware.calculate_optimal_params(None);
        assert_eq!(params.memory_buffer_percent, 0.3); // 30% buffer

        // Test 8GB system (25% buffer)
        let medium_memory_hardware = SystemHardware {
            total_memory: 6 * 1024 * 1024 * 1024, // 6GB (less than 8GB)
            available_memory: 5 * 1024 * 1024 * 1024,
            cpu_cores: 4,
            cpu_threads: 8,
            cpu_model: "Medium CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = medium_memory_hardware.calculate_optimal_params(None);
        assert_eq!(params.memory_buffer_percent, 0.25); // 25% buffer

        // Test 16GB system (20% buffer)
        let good_memory_hardware = SystemHardware {
            total_memory: 12 * 1024 * 1024 * 1024, // 12GB (less than 16GB)
            available_memory: 10 * 1024 * 1024 * 1024,
            cpu_cores: 6,
            cpu_threads: 12,
            cpu_model: "Good CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let params = good_memory_hardware.calculate_optimal_params(None);
        assert_eq!(params.memory_buffer_percent, 0.2); // 20% buffer
    }

    #[test]
    fn test_system_hardware_clone() {
        let hardware = SystemHardware {
            total_memory: 16 * 1024 * 1024 * 1024,
            available_memory: 12 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "Test CPU".to_string(),
            gpu_type: GpuType::Nvidia,
            gpu_memory: Some(8 * 1024 * 1024 * 1024),
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        let cloned = hardware.clone();
        assert_eq!(hardware.total_memory, cloned.total_memory);
        assert_eq!(hardware.cpu_model, cloned.cpu_model);
        assert_eq!(hardware.gpu_type, cloned.gpu_type);
    }

    #[test]
    fn test_optimal_params_clone() {
        let params = OptimalParams {
            n_threads: 8,
            n_gpu_layers: 20,
            context_size: 2048,
            batch_size: 512,
            max_model_size: 8 * 1024 * 1024 * 1024,
            use_mmap: true,
            memory_buffer_percent: 0.2,
        };

        let cloned = params.clone();
        assert_eq!(params.n_threads, cloned.n_threads);
        assert_eq!(params.n_gpu_layers, cloned.n_gpu_layers);
        assert_eq!(params.context_size, cloned.context_size);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_memory_macos() {
        // Test that we can call the memory detection function
        // This will actually call the system commands on macOS
        let result = SystemHardware::detect_memory();
        match result {
            Ok((total, available)) => {
                assert!(total > 0);
                assert!(available <= total);
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("Memory detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_detect_memory_linux() {
        // Test that we can call the memory detection function
        // This will actually call the system commands on Linux
        let result = SystemHardware::detect_memory();
        match result {
            Ok((total, available)) => {
                assert!(total > 0);
                assert!(available <= total);
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("Memory detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_cpu_macos() {
        // Test that we can call the CPU detection function
        let result = SystemHardware::detect_cpu();
        match result {
            Ok((cores, threads, model)) => {
                assert!(cores > 0);
                assert!(threads >= cores);
                assert!(!model.is_empty());
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("CPU detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_detect_cpu_linux() {
        // Test that we can call the CPU detection function
        let result = SystemHardware::detect_cpu();
        match result {
            Ok((cores, threads, model)) => {
                assert!(cores > 0);
                assert!(threads >= cores);
                assert!(!model.is_empty());
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("CPU detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_detect_gpu_macos() {
        // Test that we can call the GPU detection function
        let result = SystemHardware::detect_gpu();
        match result {
            Ok((gpu_type, _memory)) => {
                // Should detect some type of GPU
                assert!(gpu_type != GpuType::None || gpu_type == GpuType::None);
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("GPU detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_detect_gpu_linux() {
        // Test that we can call the GPU detection function
        let result = SystemHardware::detect_gpu();
        match result {
            Ok((gpu_type, _memory)) => {
                // Should detect some type of GPU
                assert!(gpu_type != GpuType::None || gpu_type == GpuType::None);
            }
            Err(e) => {
                // It's okay if it fails in CI environments
                eprintln!("GPU detection failed (expected in CI): {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_unsupported_platform_windows() {
        // Test that Windows returns UnsupportedPlatform error
        let memory_result = SystemHardware::detect_memory();
        assert!(matches!(
            memory_result,
            Err(HardwareError::UnsupportedPlatform)
        ));

        let cpu_result = SystemHardware::detect_cpu();
        assert!(matches!(
            cpu_result,
            Err(HardwareError::UnsupportedPlatform)
        ));

        let gpu_result = SystemHardware::detect_gpu();
        assert!(matches!(
            gpu_result,
            Err(HardwareError::UnsupportedPlatform)
        ));
    }

    #[test]
    fn test_detect_fresh() {
        // Test the detect_fresh method
        let result = SystemHardware::detect_fresh();
        match result {
            Ok(hardware) => {
                assert!(hardware.total_memory > 0);
                assert!(hardware.cpu_cores > 0);
                assert!(!hardware.cpu_model.is_empty());
                assert!(!hardware.architecture.is_empty());
            }
            Err(e) => {
                // It's okay if it fails in CI environments or unsupported platforms
                eprintln!(
                    "Hardware detection failed (expected in CI or unsupported platform): {:?}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_detect_with_caching() {
        // Test the main detect method which uses caching
        let result1 = SystemHardware::detect();
        let result2 = SystemHardware::detect();

        match (result1, result2) {
            (Ok(hardware1), Ok(hardware2)) => {
                // Should get the same results due to caching
                assert_eq!(hardware1.total_memory, hardware2.total_memory);
                assert_eq!(hardware1.cpu_cores, hardware2.cpu_cores);
                assert_eq!(hardware1.cpu_model, hardware2.cpu_model);
            }
            (Err(_), Err(_)) => {
                // Both failed, which is okay in CI environments
                eprintln!("Hardware detection failed consistently (expected in CI)");
            }
            _ => {
                panic!("Hardware detection results should be consistent");
            }
        }
    }

    #[test]
    fn test_context_size_calculation_edge_cases() {
        let hardware = SystemHardware {
            total_memory: 32 * 1024 * 1024 * 1024,     // 32GB
            available_memory: 28 * 1024 * 1024 * 1024, // 28GB available
            cpu_cores: 8,
            cpu_threads: 16,
            cpu_model: "High-end CPU".to_string(),
            gpu_type: GpuType::None,
            gpu_memory: None,
            supports_mmap: true,
            architecture: "x86_64".to_string(),
        };

        // Available for model: 28GB * 0.85 = 23.8GB

        // Test with a model that uses most available memory
        let large_model = 20 * 1024 * 1024 * 1024; // 20GB model
        let params = hardware.calculate_optimal_params(Some(large_model));
        // Remaining: 23.8GB - 20GB = 3.8GB (>=2GB but <4GB), so should get 4096 context
        assert_eq!(params.context_size, 4096);

        // Test with very large remaining memory (4GB+)
        let small_model = 4 * 1024 * 1024 * 1024; // 4GB model
        let params = hardware.calculate_optimal_params(Some(small_model));
        // Remaining: 23.8GB - 4GB = 19.8GB (>4GB), so should get 8192 context
        assert_eq!(params.context_size, 8192);

        // Test with medium remaining memory (2GB+)
        let medium_model = 19 * 1024 * 1024 * 1024; // 19GB model
        let params = hardware.calculate_optimal_params(Some(medium_model));
        // Remaining: 23.8GB - 19GB = 4.8GB (>4GB), so should get 8192 context
        assert_eq!(params.context_size, 8192);

        // Test with small remaining memory (1GB+)
        let large_model2 = 22 * 1024 * 1024 * 1024; // 22GB model
        let params = hardware.calculate_optimal_params(Some(large_model2));
        // Remaining: 23.8GB - 22GB = 1.8GB (>1GB), so should get 2048 context
        assert_eq!(params.context_size, 2048);
    }
}
