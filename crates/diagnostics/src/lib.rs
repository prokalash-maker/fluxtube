use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::{get_current_pid, ProcessesToUpdate, System};

pub struct LoggingGuard {
    _worker_guard: tracing_appender::non_blocking::WorkerGuard,
}

pub fn init_logging(
    log_dir: Option<&Path>,
    default_filter: Option<&str>,
) -> Result<LoggingGuard, Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter_str = default_filter.unwrap_or("info,fluxtube=debug,wry=warn,wgpu=warn");
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter_str));

    let (writer, guard) = if let Some(dir) = log_dir {
        let file_appender = tracing_appender::rolling::daily(dir, "fluxtube.log");
        tracing_appender::non_blocking(file_appender)
    } else {
        tracing_appender::non_blocking(std::io::stdout())
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(log_dir.is_none());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()?;

    Ok(LoggingGuard {
        _worker_guard: guard,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub process_memory_mb: u64,
    pub os_name: String,
    pub os_version: String,
    pub cpu_arch: String,
    pub cpu_cores: usize,
    pub cpu_brand: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticReport {
    pub generated_at: DateTime<Utc>,
    pub app_version: String,
    pub build_mode: String,
    pub metrics: SystemMetrics,
    pub active_profile: String,
    pub active_theme: String,
    pub active_video_backend: String,
    pub active_graphics_backend: String,
    pub hardware_decoding: bool,
    pub auth_status: String,
}

impl DiagnosticReport {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_markdown(&self) -> String {
        format!(
            "# FluxTube Diagnostics Report\n\
            **Generated At:** {}\n\
            **App Version:** {} ({})\n\n\
            ## System Telemetry\n\
            - **OS:** {} {}\n\
            - **CPU Architecture:** {}\n\
            - **CPU Brand:** {}\n\
            - **Logical Cores:** {}\n\
            - **CPU Usage:** {:.1}%\n\
            - **System RAM:** {} MB / {} MB\n\
            - **FluxTube RAM (RSS):** {} MB\n\
            - **System Uptime:** {}s\n\n\
            ## Application Configuration\n\
            - **Performance Profile:** {}\n\
            - **UI Theme:** {}\n\
            - **Video Backend:** {}\n\
            - **Graphics Backend:** {}\n\
            - **Hardware Acceleration:** {}\n\
            - **Auth Status:** {}\n",
            self.generated_at,
            self.app_version,
            self.build_mode,
            self.metrics.os_name,
            self.metrics.os_version,
            self.metrics.cpu_arch,
            self.metrics.cpu_brand,
            self.metrics.cpu_cores,
            self.metrics.cpu_usage_percent,
            self.metrics.memory_used_mb,
            self.metrics.memory_total_mb,
            self.metrics.process_memory_mb,
            self.metrics.uptime_seconds,
            self.active_profile,
            self.active_theme,
            self.active_video_backend,
            self.active_graphics_backend,
            if self.hardware_decoding {
                "Enabled (H.264)"
            } else {
                "Disabled / Software"
            },
            self.auth_status
        )
    }
}

pub struct DiagnosticsCollector {
    sys: System,
    os_name: String,
    os_version: String,
    cpu_arch: String,
    cpu_cores: usize,
    cpu_brand: String,
}

impl DiagnosticsCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let cpu_arch = System::cpu_arch().unwrap_or_else(|| "Unknown".to_string());
        let cpu_cores = sys.cpus().len();
        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Self {
            sys,
            os_name,
            os_version,
            cpu_arch,
            cpu_cores,
            cpu_brand,
        }
    }

    pub fn collect_metrics(&mut self) -> SystemMetrics {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let process_memory_mb = if let Ok(pid) = get_current_pid() {
            self.sys
                .refresh_processes(ProcessesToUpdate::Some(&[pid]));
            self.sys
                .process(pid)
                .map(|p| p.memory() / (1024 * 1024))
                .unwrap_or(0)
        } else {
            0
        };

        SystemMetrics {
            cpu_usage_percent: self.sys.global_cpu_usage(),
            memory_used_mb: self.sys.used_memory() / (1024 * 1024),
            memory_total_mb: self.sys.total_memory() / (1024 * 1024),
            process_memory_mb,
            os_name: self.os_name.clone(),
            os_version: self.os_version.clone(),
            cpu_arch: self.cpu_arch.clone(),
            cpu_cores: self.cpu_cores,
            cpu_brand: self.cpu_brand.clone(),
            uptime_seconds: System::uptime(),
        }
    }

    pub fn generate_report(
        &mut self,
        app_version: &str,
        active_profile: &str,
        active_theme: &str,
        active_video_backend: &str,
        active_graphics_backend: &str,
        hardware_decoding: bool,
        is_authenticated: bool,
    ) -> DiagnosticReport {
        let metrics = self.collect_metrics();
        let build_mode = if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        };

        DiagnosticReport {
            generated_at: Utc::now(),
            app_version: app_version.to_string(),
            build_mode,
            metrics,
            active_profile: active_profile.to_string(),
            active_theme: active_theme.to_string(),
            active_video_backend: active_video_backend.to_string(),
            active_graphics_backend: active_graphics_backend.to_string(),
            hardware_decoding,
            auth_status: if is_authenticated {
                "Authenticated (Google OAuth)".to_string()
            } else {
                "Guest Mode".to_string()
            },
        }
    }
}

impl Default for DiagnosticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_metrics() {
        let mut collector = DiagnosticsCollector::new();
        let metrics = collector.collect_metrics();

        assert!(!metrics.os_name.is_empty());
        assert!(!metrics.cpu_arch.is_empty());
        assert!(metrics.cpu_cores > 0);
        assert!(metrics.memory_total_mb > 0);
    }

    #[test]
    fn test_generate_report_and_privacy_redaction() {
        let mut collector = DiagnosticsCollector::new();
        let report = collector.generate_report(
            "0.1.0",
            "Balanced",
            "graphite_dark",
            "wry (WKWebView / WebView2)",
            "wgpu (DirectX 12)",
            true,
            false,
        );

        let markdown = report.to_markdown();
        assert!(markdown.contains("FluxTube Diagnostics Report"));
        assert!(markdown.contains("Balanced"));
        assert!(markdown.contains("Guest Mode"));

        // Verify privacy: no personal tokens, email, or credentials present
        assert!(!markdown.contains("@"));
        assert!(!markdown.contains("token"));
        assert!(!markdown.contains("Bearer"));

        let json = report.to_json().expect("to_json");
        assert!(json.contains("\"app_version\": \"0.1.0\""));
        assert!(!json.contains("email"));
    }
}
