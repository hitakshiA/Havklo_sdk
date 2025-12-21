//! Benchmarks for JSON message parsing
//!
//! Run with: cargo bench --bench parsing

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use kraken_types::WsMessage;

/// Sample status message
const STATUS_MESSAGE: &str = r#"{
    "channel": "status",
    "type": "update",
    "data": [{
        "api_version": "v2",
        "connection_id": 12345678901234567890,
        "system": "online",
        "version": "2.0.10"
    }]
}"#;

/// Sample heartbeat message
const HEARTBEAT_MESSAGE: &str = r#"{"channel": "heartbeat"}"#;

/// Sample book snapshot with 10 levels each side
const BOOK_SNAPSHOT_10: &str = r#"{
    "channel": "book",
    "type": "snapshot",
    "data": [{
        "symbol": "BTC/USD",
        "bids": [
            {"price": 100000.0, "qty": 1.5},
            {"price": 99999.0, "qty": 2.0},
            {"price": 99998.0, "qty": 0.5},
            {"price": 99997.0, "qty": 1.0},
            {"price": 99996.0, "qty": 2.5},
            {"price": 99995.0, "qty": 0.75},
            {"price": 99994.0, "qty": 1.25},
            {"price": 99993.0, "qty": 3.0},
            {"price": 99992.0, "qty": 0.1},
            {"price": 99991.0, "qty": 4.0}
        ],
        "asks": [
            {"price": 100001.0, "qty": 1.0},
            {"price": 100002.0, "qty": 2.5},
            {"price": 100003.0, "qty": 0.75},
            {"price": 100004.0, "qty": 1.5},
            {"price": 100005.0, "qty": 2.0},
            {"price": 100006.0, "qty": 0.5},
            {"price": 100007.0, "qty": 1.0},
            {"price": 100008.0, "qty": 3.5},
            {"price": 100009.0, "qty": 0.25},
            {"price": 100010.0, "qty": 5.0}
        ],
        "checksum": 123456789,
        "timestamp": "2025-12-21T12:28:24.113018Z"
    }]
}"#;

/// Sample book update (small)
const BOOK_UPDATE_SMALL: &str = r#"{
    "channel": "book",
    "type": "update",
    "data": [{
        "symbol": "BTC/USD",
        "bids": [{"price": 100000.0, "qty": 2.0}],
        "asks": [],
        "checksum": 987654321,
        "timestamp": "2025-12-21T12:28:24.321740Z"
    }]
}"#;

/// Sample subscribe response
const SUBSCRIBE_RESPONSE: &str = r#"{
    "method": "subscribe",
    "req_id": 1,
    "result": {
        "channel": "book",
        "depth": 10,
        "snapshot": true,
        "symbol": "BTC/USD"
    },
    "success": true,
    "time_in": "2025-12-21T12:28:24.000000Z",
    "time_out": "2025-12-21T12:28:24.001000Z"
}"#;

fn bench_parse_status(c: &mut Criterion) {
    c.bench_function("parse_status_message", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(STATUS_MESSAGE));
            black_box(result)
        })
    });
}

fn bench_parse_heartbeat(c: &mut Criterion) {
    c.bench_function("parse_heartbeat", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(HEARTBEAT_MESSAGE));
            black_box(result)
        })
    });
}

fn bench_parse_book_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_book_snapshot");
    group.throughput(Throughput::Bytes(BOOK_SNAPSHOT_10.len() as u64));

    group.bench_function("10_levels", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(BOOK_SNAPSHOT_10));
            black_box(result)
        })
    });

    group.finish();
}

fn bench_parse_book_update(c: &mut Criterion) {
    c.bench_function("parse_book_update", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(BOOK_UPDATE_SMALL));
            black_box(result)
        })
    });
}

fn bench_parse_subscribe_response(c: &mut Criterion) {
    c.bench_function("parse_subscribe_response", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(SUBSCRIBE_RESPONSE));
            black_box(result)
        })
    });
}

fn bench_parse_with_scientific_notation(c: &mut Criterion) {
    // Message with scientific notation quantities
    let json = r#"{
        "channel": "book",
        "type": "snapshot",
        "data": [{
            "symbol": "BTC/USD",
            "bids": [
                {"price": 100000, "qty": 5e-6},
                {"price": 99999, "qty": 1.5e-4},
                {"price": 99998, "qty": 2.5e-8}
            ],
            "asks": [
                {"price": 100001, "qty": 1e-5},
                {"price": 100002, "qty": 3.14e-3}
            ],
            "checksum": 0,
            "timestamp": "2025-12-21T12:00:00Z"
        }]
    }"#;

    c.bench_function("parse_scientific_notation", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(json));
            black_box(result)
        })
    });
}

fn bench_parse_high_precision(c: &mut Criterion) {
    // Message with high-precision decimals
    let json = r#"{
        "channel": "book",
        "type": "snapshot",
        "data": [{
            "symbol": "BTC/USD",
            "bids": [
                {"price": 100000.123456789, "qty": 0.000000001},
                {"price": 99999.987654321, "qty": 0.123456789}
            ],
            "asks": [
                {"price": 100001.111111111, "qty": 0.999999999},
                {"price": 100002.222222222, "qty": 0.555555555}
            ],
            "checksum": 0,
            "timestamp": "2025-12-21T12:00:00Z"
        }]
    }"#;

    c.bench_function("parse_high_precision", |b| {
        b.iter(|| {
            let result = WsMessage::parse(black_box(json));
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_parse_status,
    bench_parse_heartbeat,
    bench_parse_book_snapshot,
    bench_parse_book_update,
    bench_parse_subscribe_response,
    bench_parse_with_scientific_notation,
    bench_parse_high_precision,
);

criterion_main!(benches);
