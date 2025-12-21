//! Benchmarks for orderbook operations
//!
//! Run with: cargo bench --bench orderbook

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kraken_book::{compute_checksum, Orderbook, TreeBook};
use kraken_types::{BookData, Level};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Create N price levels starting from base price
fn create_levels(base_price: Decimal, count: usize, step: Decimal) -> Vec<Level> {
    (0..count)
        .map(|i| {
            Level::new(
                base_price + step * Decimal::from(i as i64),
                dec!(1.0) + Decimal::from(i as i64) / dec!(10),
            )
        })
        .collect()
}

/// Create book data with correct checksum
fn create_book_data(bid_count: usize, ask_count: usize) -> BookData {
    let bids = create_levels(dec!(100000), bid_count, dec!(-1));
    let asks = create_levels(dec!(100001), ask_count, dec!(1));
    let checksum = compute_checksum(&bids, &asks);

    BookData {
        symbol: "BTC/USD".to_string(),
        bids,
        asks,
        checksum,
        timestamp: None,
    }
}

fn bench_treebook_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("treebook_insert");

    for size in [10, 100, 500, 1000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut book = TreeBook::new();
                for i in 0..size {
                    let price = Decimal::from(100000 - i);
                    let qty = Decimal::from(i + 1);
                    book.insert_bid(black_box(price), black_box(qty));
                }
                black_box(book)
            })
        });
    }

    group.finish();
}

fn bench_treebook_lookup(c: &mut Criterion) {
    // Pre-populate a book with 1000 levels
    let mut book = TreeBook::new();
    for i in 0..1000 {
        book.insert_bid(Decimal::from(100000 - i), Decimal::from(i + 1));
        book.insert_ask(Decimal::from(100001 + i), Decimal::from(i + 1));
    }

    let mut group = c.benchmark_group("treebook_lookup");

    group.bench_function("best_bid", |b| {
        b.iter(|| {
            let result = book.best_bid();
            black_box(result)
        })
    });

    group.bench_function("best_ask", |b| {
        b.iter(|| {
            let result = book.best_ask();
            black_box(result)
        })
    });

    group.bench_function("bids_vec", |b| {
        b.iter(|| {
            let result = book.bids_vec();
            black_box(result)
        })
    });

    group.bench_function("top_10_bids", |b| {
        b.iter(|| {
            let result = book.top_bids(10);
            black_box(result)
        })
    });

    group.finish();
}

fn bench_orderbook_apply_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_apply_snapshot");

    for size in [10, 25, 100, 500] {
        group.throughput(Throughput::Elements((size * 2) as u64)); // Both sides

        let data = create_book_data(size, size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| {
                let mut book = Orderbook::new("BTC/USD");
                let result = book.apply_book_data(black_box(data), true);
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_orderbook_apply_delta(c: &mut Criterion) {
    // Create initial orderbook
    let snapshot = create_book_data(100, 100);
    let mut template_book = Orderbook::new("BTC/USD");
    template_book.apply_book_data(&snapshot, true).unwrap();

    // Create a delta update (changes just a few levels)
    let delta_bids = vec![Level::new(dec!(100000), dec!(2.5))]; // Update top bid
    let delta_asks = vec![Level::new(dec!(100001), dec!(1.5))]; // Update top ask

    // Get the new checksum after applying delta
    let mut all_bids = template_book.bids_vec();
    let mut all_asks = template_book.asks_vec();
    all_bids[0] = delta_bids[0].clone();
    all_asks[0] = delta_asks[0].clone();
    let checksum = compute_checksum(&all_bids, &all_asks);

    let delta = BookData {
        symbol: "BTC/USD".to_string(),
        bids: delta_bids,
        asks: delta_asks,
        checksum,
        timestamp: None,
    };

    c.bench_function("orderbook_apply_delta", |b| {
        b.iter_batched(
            || {
                let mut book = Orderbook::new("BTC/USD");
                book.apply_book_data(&snapshot, true).unwrap();
                book
            },
            |mut book| {
                let result = book.apply_book_data(black_box(&delta), false);
                black_box(result)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_checksum_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum_compute");

    for size in [10, 25, 100] {
        let bids = create_levels(dec!(100000), size, dec!(-1));
        let asks = create_levels(dec!(100001), size, dec!(1));

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(bids.clone(), asks.clone()),
            |b, (bids, asks)| {
                b.iter(|| {
                    let result = compute_checksum(black_box(bids), black_box(asks));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_spread_calculation(c: &mut Criterion) {
    let data = create_book_data(100, 100);
    let mut book = Orderbook::new("BTC/USD");
    book.apply_book_data(&data, true).unwrap();

    let mut group = c.benchmark_group("calculations");

    group.bench_function("spread", |b| {
        b.iter(|| {
            let result = book.spread();
            black_box(result)
        })
    });

    group.bench_function("mid_price", |b| {
        b.iter(|| {
            let result = book.mid_price();
            black_box(result)
        })
    });

    group.finish();
}

fn bench_orderbook_snapshot(c: &mut Criterion) {
    let data = create_book_data(100, 100);
    let mut book = Orderbook::new("BTC/USD");
    book.apply_book_data(&data, true).unwrap();

    c.bench_function("orderbook_snapshot_capture", |b| {
        b.iter(|| {
            let result = book.snapshot();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_treebook_insert,
    bench_treebook_lookup,
    bench_orderbook_apply_snapshot,
    bench_orderbook_apply_delta,
    bench_checksum_compute,
    bench_spread_calculation,
    bench_orderbook_snapshot,
);

criterion_main!(benches);
