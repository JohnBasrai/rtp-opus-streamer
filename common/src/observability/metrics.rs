//! Prometheus metrics (Rust `prometheus` crate).
//!
//! One `MetricsContext` is intended per process. Each binary owns its registry
//! and controls which metrics it reports.

use anyhow::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Configuration for the built-in Prometheus scrape endpoint.
#[derive(Debug, Clone)]
pub struct MetricsServerConfig {
    // ---
    /// Address to bind, e.g. `127.0.0.1:9100`.
    pub bind: SocketAddr,
}

impl MetricsServerConfig {
    // ---
    pub fn new(bind: SocketAddr) -> Self {
        // ---
        Self { bind }
    }
}

/// Prometheus metrics registry + handles.
///
/// This is a thin, explicit wrapper around the `prometheus` crate so hot-path
/// instrumentation is just counter increments / histogram observations.
#[derive(Clone)]
pub struct MetricsContext {
    // ---
    registry: Registry,

    // Network counters
    pub packets_sent_total: IntCounter,
    pub packets_received_total: IntCounter,
    pub packets_lost_total: IntCounter,
    pub packets_reordered_total: IntCounter,
    pub packets_late_total: IntCounter,

    pub bytes_sent_total: IntCounter,
    pub bytes_received_total: IntCounter,

    // Buffer gauges
    pub jitter_buffer_occupancy_packets: IntGauge,

    // Latency histograms (seconds)
    pub encode_seconds: Histogram,
    pub decode_seconds: Histogram,
    pub jitter_buffer_delay_seconds: Histogram,
    pub network_transit_seconds: Histogram,
    pub receiver_pipeline_seconds: Histogram,
}

impl MetricsContext {
    // ---
    /// Create a new registry and register the standard metrics.
    ///
    /// `process_name` is applied as a constant label (`process=<name>`).
    pub fn new(process_name: &str) -> Result<Self> {
        // ---
        let registry = Registry::new_custom(
            Some("rtp_opus_streamer".into()),
            Some(prometheus::labels! { "process".to_string() => process_name.to_string() }),
        )?;

        let packets_sent_total = IntCounter::with_opts(Opts::new(
            "rtp_packets_sent_total",
            "Total RTP packets sent",
        ))?;
        let packets_received_total = IntCounter::with_opts(Opts::new(
            "rtp_packets_received_total",
            "Total RTP packets received",
        ))?;
        let packets_lost_total = IntCounter::with_opts(Opts::new(
            "rtp_packets_lost_total",
            "Total RTP packets detected as lost",
        ))?;
        let packets_reordered_total = IntCounter::with_opts(Opts::new(
            "rtp_packets_reordered_total",
            "Total RTP packets received out of order",
        ))?;
        let packets_late_total = IntCounter::with_opts(Opts::new(
            "rtp_packets_late_total",
            "Total RTP packets that arrived too late for playout",
        ))?;

        let bytes_sent_total = IntCounter::with_opts(Opts::new(
            "rtp_bytes_sent_total",
            "Total RTP payload bytes sent",
        ))?;
        let bytes_received_total = IntCounter::with_opts(Opts::new(
            "rtp_bytes_received_total",
            "Total RTP payload bytes received",
        ))?;

        let jitter_buffer_occupancy_packets = IntGauge::with_opts(Opts::new(
            "jitter_buffer_occupancy_packets",
            "Current jitter buffer occupancy in packets",
        ))?;

        let encode_seconds = Histogram::with_opts(HistogramOpts::new(
            "opus_encode_seconds",
            "Opus encode duration in seconds",
        ))?;
        let decode_seconds = Histogram::with_opts(HistogramOpts::new(
            "opus_decode_seconds",
            "Opus decode duration in seconds",
        ))?;
        let jitter_buffer_delay_seconds = Histogram::with_opts(HistogramOpts::new(
            "jitter_buffer_delay_seconds",
            "Time a packet spent waiting in the jitter buffer (seconds)",
        ))?;
        let network_transit_seconds = Histogram::with_opts(HistogramOpts::new(
            "network_transit_seconds",
            "Estimated network transit time (seconds)",
        ))?;
        let receiver_pipeline_seconds = Histogram::with_opts(HistogramOpts::new(
            "receiver_pipeline_seconds",
            "Receiver pipeline time from packet arrival to audio enqueue (seconds)",
        ))?;

        // Register all metrics
        registry.register(Box::new(packets_sent_total.clone()))?;
        registry.register(Box::new(packets_received_total.clone()))?;
        registry.register(Box::new(packets_lost_total.clone()))?;
        registry.register(Box::new(packets_reordered_total.clone()))?;
        registry.register(Box::new(packets_late_total.clone()))?;
        registry.register(Box::new(bytes_sent_total.clone()))?;
        registry.register(Box::new(bytes_received_total.clone()))?;
        registry.register(Box::new(jitter_buffer_occupancy_packets.clone()))?;
        registry.register(Box::new(encode_seconds.clone()))?;
        registry.register(Box::new(decode_seconds.clone()))?;
        registry.register(Box::new(jitter_buffer_delay_seconds.clone()))?;
        registry.register(Box::new(network_transit_seconds.clone()))?;
        registry.register(Box::new(receiver_pipeline_seconds.clone()))?;

        Ok(Self {
            registry,
            packets_sent_total,
            packets_received_total,
            packets_lost_total,
            packets_reordered_total,
            packets_late_total,
            bytes_sent_total,
            bytes_received_total,
            jitter_buffer_occupancy_packets,
            encode_seconds,
            decode_seconds,
            jitter_buffer_delay_seconds,
            network_transit_seconds,
            receiver_pipeline_seconds,
        })
    }

    /// Gather metric families from this registry.
    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        // ---
        self.registry.gather()
    }

    /// Spawns a minimal HTTP server that serves `GET /metrics`.
    ///
    /// This is intentionally explicit (callers decide whether to run it).
    pub fn spawn_metrics_server(&self, cfg: MetricsServerConfig) -> JoinHandle<Result<()>> {
        // ---
        let registry = Arc::new(self.registry.clone());
        tokio::spawn(async move {
            // ---
            let make_svc = make_service_fn(move |_conn| {
                let registry = Arc::clone(&registry);
                async move {
                    Ok::<_, hyper::Error>(service_fn(move |req| {
                        let registry = Arc::clone(&registry);
                        async move { handle_metrics_request(req, registry).await }
                    }))
                }
            });

            let server = Server::bind(&cfg.bind).serve(make_svc);
            server.await.map_err(|e| anyhow::anyhow!(e))?;
            Ok(())
        })
    }
}

async fn handle_metrics_request(
    req: Request<Body>,
    registry: Arc<Registry>,
) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let encoder = TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = Vec::new();

            if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
                let mut resp = Response::new(Body::from(format!("encode error: {e}")));
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return Ok(resp);
            }

            let mut resp = Response::new(Body::from(buffer));
            resp.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                hyper::header::HeaderValue::from_static("text/plain; version=0.0.4"),
            );
            Ok(resp)
        }
        _ => {
            let mut resp = Response::new(Body::from("not found"));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}

#[cfg(test)]
mod tests {
    // ---
    use super::*;

    #[test]
    fn metrics_context_gathers_something() {
        // ---
        let ctx = MetricsContext::new("test").expect("MetricsContext should init");
        let families = ctx.gather();
        assert!(!families.is_empty());
    }
}
