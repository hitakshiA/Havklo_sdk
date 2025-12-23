//! Benchmarks for L3 (Level 3) orderbook operations
//!
//! Run with: cargo bench --bench l3_orderbook

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kraken_book::l3::{L3Book, L3Order, L3Side};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Create an L3 book with N orders on each side
fn create_l3_book(orders_per_side: usize) -> L3Book {
    let mut book = L3Book::new("BTC/USD", 1000);

    // Add bid orders at different price levels
    for i in 0..orders_per_side {
        let price = dec!(100000) - Decimal::from(i as i64 / 10); // 10 orders per level
        let qty = dec!(0.1) + Decimal::from(i as i64) / dec!(1000);
        let order_id = format!("bid_{}", i);
        let order = L3Order::new(order_id, price, qty);
        book.add_order(order, L3Side::Bid);
    }

    // Add ask orders at different price levels
    for i in 0..orders_per_side {
        let price = dec!(100001) + Decimal::from(i as i64 / 10);
        let qty = dec!(0.1) + Decimal::from(i as i64) / dec!(1000);
        let order_id = format!("ask_{}", i);
        let order = L3Order::new(order_id, price, qty);
        book.add_order(order, L3Side::Ask);
    }

    book
}

fn bench_l3_add_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("l3_add_order");

    for size in [100, 500, 1000] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || create_l3_book(size),
                |mut book| {
                    let order = L3Order::new("new_order", dec!(100000.5), dec!(1.0));
                    book.add_order(black_box(order), black_box(L3Side::Bid));
                    black_box(book)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_l3_remove_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("l3_remove_order");

    for size in [100, 500, 1000] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || create_l3_book(size),
                |mut book| {
                    // Remove an order from the middle
                    let result = book.remove_order(black_box(&format!("bid_{}", size / 2)));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_l3_modify_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("l3_modify_order");

    for size in [100, 500, 1000] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || create_l3_book(size),
                |mut book| {
                    // Modify order quantity
                    let result = book.modify_order(
                        black_box(&format!("bid_{}", size / 2)),
                        black_box(dec!(5.0)),
                    );
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_l3_queue_position(c: &mut Criterion) {
    // Create book with many orders at same price level
    let mut book = L3Book::new("BTC/USD", 1000);
    for i in 0..100 {
        let order_id = format!("order_{}", i);
        let order = L3Order::new(order_id, dec!(100000), dec!(1.0));
        book.add_order(order, L3Side::Bid);
    }

    let mut group = c.benchmark_group("l3_queue_position");

    // Query position of order in the middle of the queue
    group.bench_function("middle_of_queue", |b| {
        b.iter(|| {
            let result = book.queue_position(black_box("order_50"));
            black_box(result)
        })
    });

    // Query position of order at front
    group.bench_function("front_of_queue", |b| {
        b.iter(|| {
            let result = book.queue_position(black_box("order_0"));
            black_box(result)
        })
    });

    group.finish();
}

fn bench_l3_aggregated_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("l3_aggregated_levels");

    for size in [100, 500, 1000] {
        let book = create_l3_book(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &book, |b, book| {
            b.iter(|| {
                let result = book.aggregated_bids();
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_l3_best_bid_ask(c: &mut Criterion) {
    let book = create_l3_book(1000);

    let mut group = c.benchmark_group("l3_bbo");

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

    group.finish();
}

fn bench_l3_vwap(c: &mut Criterion) {
    let book = create_l3_book(500);

    let mut group = c.benchmark_group("l3_vwap");

    for size in [dec!(1), dec!(10), dec!(100)] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size.to_string()),
            &size,
            |b, &size| {
                b.iter(|| {
                    let result = book.vwap_bid(black_box(size));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_l3_snapshot(c: &mut Criterion) {
    let book = create_l3_book(500);

    c.bench_function("l3_snapshot", |b| {
        b.iter(|| {
            let result = book.snapshot();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    bench_l3_add_order,
    bench_l3_remove_order,
    bench_l3_modify_order,
    bench_l3_queue_position,
    bench_l3_aggregated_levels,
    bench_l3_best_bid_ask,
    bench_l3_vwap,
    bench_l3_snapshot,
);

criterion_main!(benches);
