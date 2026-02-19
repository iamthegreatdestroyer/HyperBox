//! OpenTelemetry span generation from syscall traces
//!
//! Converts raw syscall and network traces from eBPF into structured OpenTelemetry spans
//! with proper parent-child relationships, attributes, and event markers.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Span context representing a single operation trace
#[derive(Debug, Clone)]
pub struct SpanContext {
    /// Unique span identifier (hex)
    pub span_id: String,
    /// Parent span identifier (hex), if any
    pub parent_span_id: Option<String>,
    /// Trace identifier (hex)
    pub trace_id: String,
    /// Operation name
    pub operation: String,
    /// Start time (Unix nanoseconds)
    pub start_time_ns: u64,
    /// End time (Unix nanoseconds)
    pub end_time_ns: u64,
    /// Span status: OK, ERROR, UNSET
    pub status: SpanStatus,
    /// Span attributes (key-value pairs)
    pub attributes: HashMap<String, String>,
    /// Span events with timestamps
    pub events: Vec<SpanEvent>,
}

/// Span execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Span completed successfully
    Ok,
    /// Span encountered an error
    Error,
    /// Status not set
    Unset,
}

impl SpanStatus {
    /// Get status name
    pub fn name(&self) -> &'static str {
        match self {
            SpanStatus::Ok => "OK",
            SpanStatus::Error => "ERROR",
            SpanStatus::Unset => "UNSET",
        }
    }
}

/// Event within a span
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp (Unix nanoseconds)
    pub timestamp_ns: u64,
    /// Event attributes
    pub attributes: HashMap<String, String>,
}

/// Syscall attribute set
#[derive(Debug, Clone)]
pub struct SyscallAttributes {
    /// Syscall name
    pub syscall_name: String,
    /// Syscall number
    pub syscall_number: u32,
    /// Return value
    pub return_value: i64,
    /// Duration in microseconds
    pub duration_us: u64,
}

/// Network I/O attribute set
#[derive(Debug, Clone)]
pub struct NetworkAttributes {
    /// Source IP address
    pub source_ip: String,
    /// Destination IP address
    pub dest_ip: String,
    /// Source port
    pub source_port: u16,
    /// Destination port
    pub dest_port: u16,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Protocol name (tcp, udp)
    pub protocol: String,
}

/// Generator for converting traces to spans
pub struct SpanGenerator {
    /// Current trace ID (hex)
    current_trace_id: String,
    /// Counter for generating unique span IDs
    span_counter: u64,
    /// Map of syscall traces to span context
    trace_spans: HashMap<String, SpanContext>,
    /// Max trace duration (ms) before creating new trace
    max_trace_duration_ms: u64,
}

impl SpanGenerator {
    /// Create new span generator
    pub fn new(trace_id: String) -> Self {
        Self {
            current_trace_id: trace_id,
            span_counter: 0,
            trace_spans: HashMap::new(),
            max_trace_duration_ms: 5000, // 5 second traces
        }
    }

    /// Generate a unique span ID
    fn generate_span_id(&mut self) -> String {
        self.span_counter += 1;
        format!("{:016x}", self.span_counter)
    }

    /// Create syscall span
    pub fn create_syscall_span(
        &mut self,
        syscall_attrs: SyscallAttributes,
        parent_span_id: Option<String>,
    ) -> SpanContext {
        let span_id = self.generate_span_id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let start_time_ns = now.as_nanos() as u64;
        let end_time_ns = start_time_ns + (syscall_attrs.duration_us * 1000) as u64;

        let status = if syscall_attrs.return_value < 0 {
            SpanStatus::Error
        } else {
            SpanStatus::Ok
        };

        let mut attributes = HashMap::new();
        attributes.insert("syscall.name".to_string(), syscall_attrs.syscall_name.clone());
        attributes.insert("syscall.number".to_string(), syscall_attrs.syscall_number.to_string());
        attributes.insert("syscall.return_value".to_string(), syscall_attrs.return_value.to_string());
        attributes.insert("syscall.duration_us".to_string(), syscall_attrs.duration_us.to_string());

        SpanContext {
            span_id,
            parent_span_id,
            trace_id: self.current_trace_id.clone(),
            operation: format!("syscall.{}", syscall_attrs.syscall_name),
            start_time_ns,
            end_time_ns,
            status,
            attributes,
            events: vec![],
        }
    }

    /// Create network I/O span
    pub fn create_network_span(
        &mut self,
        net_attrs: NetworkAttributes,
        parent_span_id: Option<String>,
    ) -> SpanContext {
        let span_id = self.generate_span_id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let start_time_ns = now.as_nanos() as u64;
        let end_time_ns = start_time_ns + 1_000_000; // 1ms default

        let mut attributes = HashMap::new();
        attributes.insert("network.source_ip".to_string(), net_attrs.source_ip);
        attributes.insert("network.dest_ip".to_string(), net_attrs.dest_ip);
        attributes.insert("network.source_port".to_string(), net_attrs.source_port.to_string());
        attributes.insert("network.dest_port".to_string(), net_attrs.dest_port.to_string());
        attributes.insert("network.bytes_sent".to_string(), net_attrs.bytes_sent.to_string());
        attributes.insert("network.bytes_received".to_string(), net_attrs.bytes_received.to_string());
        attributes.insert("network.protocol".to_string(), net_attrs.protocol);

        SpanContext {
            span_id,
            parent_span_id,
            trace_id: self.current_trace_id.clone(),
            operation: "network.io".to_string(),
            start_time_ns,
            end_time_ns,
            status: SpanStatus::Ok,
            attributes,
            events: vec![],
        }
    }

    /// Add event to span
    pub fn add_span_event(
        &mut self,
        span_id: &str,
        event_name: String,
        attributes: HashMap<String, String>,
    ) -> anyhow::Result<()> {
        // Find span in trace_spans
        for span in self.trace_spans.values_mut() {
            if span.span_id == span_id {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                span.events.push(SpanEvent {
                    name: event_name,
                    timestamp_ns: now.as_nanos() as u64,
                    attributes,
                });
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Span not found: {}", span_id))
    }

    /// Store span in trace
    pub fn store_span(&mut self, span: SpanContext) {
        self.trace_spans.insert(span.span_id.clone(), span);
    }

    /// Get span context
    pub fn get_span(&self, span_id: &str) -> Option<&SpanContext> {
        self.trace_spans.values().find(|s| s.span_id == span_id)
    }

    /// List all spans in current trace
    pub fn list_spans(&self) -> Vec<&SpanContext> {
        self.trace_spans.values().collect()
    }

    /// Clear all spans
    pub fn clear_spans(&mut self) {
        self.trace_spans.clear();
        self.span_counter = 0;
    }

    /// Rotate to new trace (with new trace ID)
    pub fn rotate_trace(&mut self, new_trace_id: String) {
        self.current_trace_id = new_trace_id;
        self.trace_spans.clear();
        self.span_counter = 0;
    }

    /// Get trace ID
    pub fn trace_id(&self) -> &str {
        &self.current_trace_id
    }

    /// Get span count
    pub fn span_count(&self) -> usize {
        self.trace_spans.len()
    }
}

impl Default for SpanGenerator {
    fn default() -> Self {
        let trace_id = format!("{:016x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        );
        Self::new(trace_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_status_names() {
        assert_eq!(SpanStatus::Ok.name(), "OK");
        assert_eq!(SpanStatus::Error.name(), "ERROR");
        assert_eq!(SpanStatus::Unset.name(), "UNSET");
    }

    #[test]
    fn test_span_generator_creation() {
        let gen = SpanGenerator::new("abc123".to_string());
        assert_eq!(gen.trace_id(), "abc123");
        assert_eq!(gen.span_count(), 0);
    }

    #[test]
    fn test_span_generator_default() {
        let gen = SpanGenerator::default();
        assert!(!gen.trace_id().is_empty());
        assert_eq!(gen.span_count(), 0);
    }

    #[test]
    fn test_generate_unique_span_ids() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let id1 = gen.generate_span_id();
        let id2 = gen.generate_span_id();
        let id3 = gen.generate_span_id();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(id1, "0000000000000001");
        assert_eq!(id2, "0000000000000002");
        assert_eq!(id3, "0000000000000003");
    }

    #[test]
    fn test_create_syscall_span_success() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let attrs = SyscallAttributes {
            syscall_name: "open".to_string(),
            syscall_number: 2,
            return_value: 5,
            duration_us: 1500,
        };

        let span = gen.create_syscall_span(attrs, None);

        assert_eq!(span.operation, "syscall.open");
        assert_eq!(span.status, SpanStatus::Ok);
        assert_eq!(span.parent_span_id, None);
        assert!(span.attributes.contains_key("syscall.name"));
        assert_eq!(span.attributes.get("syscall.return_value").unwrap(), "5");
    }

    #[test]
    fn test_create_syscall_span_error() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let attrs = SyscallAttributes {
            syscall_name: "open".to_string(),
            syscall_number: 2,
            return_value: -1,
            duration_us: 500,
        };

        let span = gen.create_syscall_span(attrs, None);

        assert_eq!(span.status, SpanStatus::Error);
    }

    #[test]
    fn test_create_network_span() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let attrs = NetworkAttributes {
            source_ip: "127.0.0.1".to_string(),
            dest_ip: "192.168.1.1".to_string(),
            source_port: 12345,
            dest_port: 80,
            bytes_sent: 1024,
            bytes_received: 4096,
            protocol: "tcp".to_string(),
        };

        let span = gen.create_network_span(attrs, None);

        assert_eq!(span.operation, "network.io");
        assert_eq!(span.status, SpanStatus::Ok);
        assert_eq!(span.attributes.get("network.source_ip").unwrap(), "127.0.0.1");
        assert_eq!(span.attributes.get("network.dest_port").unwrap(), "80");
    }

    #[test]
    fn test_store_and_retrieve_span() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let attrs = SyscallAttributes {
            syscall_name: "read".to_string(),
            syscall_number: 0,
            return_value: 256,
            duration_us: 1000,
        };

        let span = gen.create_syscall_span(attrs, None);
        let span_id = span.span_id.clone();
        gen.store_span(span);

        assert_eq!(gen.span_count(), 1);
        assert!(gen.get_span(&span_id).is_some());
    }

    #[test]
    fn test_add_span_event() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let attrs = SyscallAttributes {
            syscall_name: "write".to_string(),
            syscall_number: 1,
            return_value: 128,
            duration_us: 2000,
        };

        let span = gen.create_syscall_span(attrs, None);
        let span_id = span.span_id.clone();
        gen.store_span(span);

        let mut event_attrs = HashMap::new();
        event_attrs.insert("fd".to_string(), "3".to_string());

        assert!(gen.add_span_event(&span_id, "data_written".to_string(), event_attrs).is_ok());

        let span = gen.get_span(&span_id).unwrap();
        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "data_written");
    }

    #[test]
    fn test_parent_span_relationship() {
        let mut gen = SpanGenerator::new("trace1".to_string());

        let parent_attrs = SyscallAttributes {
            syscall_name: "open".to_string(),
            syscall_number: 2,
            return_value: 5,
            duration_us: 1000,
        };
        let parent_span = gen.create_syscall_span(parent_attrs, None);
        let parent_id = parent_span.span_id.clone();
        gen.store_span(parent_span);

        let child_attrs = SyscallAttributes {
            syscall_name: "read".to_string(),
            syscall_number: 0,
            return_value: 256,
            duration_us: 500,
        };
        let child_span = gen.create_syscall_span(child_attrs, Some(parent_id.clone()));

        assert_eq!(child_span.parent_span_id, Some(parent_id));
    }

    #[test]
    fn test_list_spans() {
        let mut gen = SpanGenerator::new("trace1".to_string());

        for i in 0..5 {
            let attrs = SyscallAttributes {
                syscall_name: format!("syscall{}", i),
                syscall_number: i as u32,
                return_value: 0,
                duration_us: 1000,
            };
            let span = gen.create_syscall_span(attrs, None);
            gen.store_span(span);
        }

        assert_eq!(gen.list_spans().len(), 5);
    }

    #[test]
    fn test_clear_spans() {
        let mut gen = SpanGenerator::new("trace1".to_string());

        let attrs = SyscallAttributes {
            syscall_name: "open".to_string(),
            syscall_number: 2,
            return_value: 5,
            duration_us: 1000,
        };
        let span = gen.create_syscall_span(attrs, None);
        gen.store_span(span);

        assert_eq!(gen.span_count(), 1);
        gen.clear_spans();
        assert_eq!(gen.span_count(), 0);
    }

    #[test]
    fn test_rotate_trace() {
        let mut gen = SpanGenerator::new("trace1".to_string());

        let attrs = SyscallAttributes {
            syscall_name: "open".to_string(),
            syscall_number: 2,
            return_value: 5,
            duration_us: 1000,
        };
        let span = gen.create_syscall_span(attrs, None);
        gen.store_span(span);

        assert_eq!(gen.span_count(), 1);
        gen.rotate_trace("trace2".to_string());

        assert_eq!(gen.trace_id(), "trace2");
        assert_eq!(gen.span_count(), 0);
    }

    #[test]
    fn test_add_event_to_nonexistent_span() {
        let mut gen = SpanGenerator::new("trace1".to_string());
        let mut event_attrs = HashMap::new();
        event_attrs.insert("key".to_string(), "value".to_string());

        let result = gen.add_span_event("nonexistent", "event".to_string(), event_attrs);
        assert!(result.is_err());
    }
}
