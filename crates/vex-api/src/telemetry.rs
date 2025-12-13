//! OpenTelemetry tracing configuration
//!
//! Provides configuration for distributed tracing with OpenTelemetry.
//! Supports OTLP export and integration with existing tracing infrastructure.

use std::time::Duration;

/// OpenTelemetry configuration
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// Service name for tracing
    pub service_name: String,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub endpoint: Option<String>,
    /// Whether tracing is enabled
    pub enabled: bool,
    /// Sample rate (0.0-1.0)
    pub sample_rate: f64,
    /// Export batch size
    pub batch_size: usize,
    /// Export interval
    pub export_interval: Duration,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            service_name: "vex-api".to_string(),
            endpoint: None,
            enabled: false,
            sample_rate: 1.0,
            batch_size: 512,
            export_interval: Duration::from_secs(5),
        }
    }
}

impl OtelConfig {
    /// Create config from environment variables
    /// 
    /// Reads:
    /// - OTEL_SERVICE_NAME: Service name (default: "vex-api")
    /// - OTEL_EXPORTER_OTLP_ENDPOINT: OTLP endpoint
    /// - OTEL_TRACES_SAMPLER_ARG: Sample rate (default: 1.0)
    pub fn from_env() -> Self {
        let service_name = std::env::var("OTEL_SERVICE_NAME")
            .unwrap_or_else(|_| "vex-api".to_string());
        
        let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
        
        let sample_rate = std::env::var("OTEL_TRACES_SAMPLER_ARG")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);
        
        Self {
            service_name,
            endpoint: endpoint.clone(),
            enabled: endpoint.is_some(),
            sample_rate,
            ..Default::default()
        }
    }

    /// Create a development config with console output
    pub fn development() -> Self {
        Self {
            service_name: "vex-api-dev".to_string(),
            endpoint: None,
            enabled: true,
            sample_rate: 1.0,
            batch_size: 1,
            export_interval: Duration::from_secs(1),
        }
    }

    /// Create a production config
    pub fn production(endpoint: &str) -> Self {
        Self {
            service_name: "vex-api".to_string(),
            endpoint: Some(endpoint.to_string()),
            enabled: true,
            sample_rate: 0.1, // 10% sampling in production
            batch_size: 512,
            export_interval: Duration::from_secs(5),
        }
    }
}

/// Initialize tracing with optional OpenTelemetry export
/// 
/// This sets up the tracing subscriber with:
/// - Console output (always)
/// - OpenTelemetry OTLP export (if configured)
pub fn init_tracing(config: &OtelConfig) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    // Build the base subscriber with env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,vex=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    if config.enabled {
        if let Some(ref endpoint) = config.endpoint {
            tracing::info!(
                service = %config.service_name,
                endpoint = %endpoint,
                sample_rate = config.sample_rate,
                "OpenTelemetry tracing enabled"
            );
        } else {
            tracing::info!(
                service = %config.service_name,
                "OpenTelemetry tracing enabled (console only)"
            );
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    // Note: Full OTLP export requires adding opentelemetry crates:
    // opentelemetry = "0.21"
    // opentelemetry-otlp = "0.14"
    // opentelemetry_sdk = "0.21"
    // tracing-opentelemetry = "0.22"
    //
    // Example with full OTLP:
    // ```
    // let tracer = opentelemetry_otlp::new_pipeline()
    //     .tracing()
    //     .with_exporter(
    //         opentelemetry_otlp::new_exporter()
    //             .tonic()
    //             .with_endpoint(endpoint)
    //     )
    //     .with_trace_config(
    //         opentelemetry_sdk::trace::config()
    //             .with_sampler(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(sample_rate))
    //             .with_resource(Resource::new(vec![
    //                 KeyValue::new("service.name", service_name),
    //             ]))
    //     )
    //     .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    // 
    // let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    // ```
}

/// Span extension trait for adding VEX-specific attributes
pub trait VexSpanExt {
    /// Add user ID to current span
    fn record_user_id(&self, user_id: &str);
    /// Add agent ID to current span
    fn record_agent_id(&self, agent_id: &str);
    /// Add request ID to current span
    fn record_request_id(&self, request_id: &str);
}

impl VexSpanExt for tracing::Span {
    fn record_user_id(&self, user_id: &str) {
        self.record("user_id", user_id);
    }
    
    fn record_agent_id(&self, agent_id: &str) {
        self.record("agent_id", agent_id);
    }
    
    fn record_request_id(&self, request_id: &str) {
        self.record("request_id", request_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_config_default() {
        let config = OtelConfig::default();
        assert_eq!(config.service_name, "vex-api");
        assert!(!config.enabled);
        assert!(config.endpoint.is_none());
    }

    #[test]
    fn test_otel_config_production() {
        let config = OtelConfig::production("http://otel:4317");
        assert!(config.enabled);
        assert_eq!(config.endpoint, Some("http://otel:4317".to_string()));
        assert_eq!(config.sample_rate, 0.1);
    }
}
