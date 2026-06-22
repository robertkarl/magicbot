use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use magicbot_engine::ids::PlayerId;
use magicbot_engine::object::{CounterKind, Zone};
use magicbot_engine::state::GameState;

fn build_state(board_size: usize) -> GameState {
    let mut s = GameState::new(2, 0);
    for i in 0..board_size {
        s.create_object(PlayerId((i % 2) as u8), Zone::Battlefield, i as u64);
    }
    s
}

fn bench_fork(c: &mut Criterion) {
    let mut group = c.benchmark_group("fork");
    // If fork() is O(1), wall-clock should be ~flat across these sizes.
    for &size in &[8usize, 64, 512, 4096] {
        let state = build_state(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &state, |b, st| {
            b.iter(|| {
                let child = st.fork();
                criterion::black_box(child);
            });
        });
    }
    group.finish();
}

fn bench_fork_then_mutate(c: &mut Criterion) {
    let mut group = c.benchmark_group("fork_then_mutate");
    for &size in &[8usize, 64, 512, 4096] {
        let state = build_state(size);
        // Grab one real id to mutate.
        let target = {
            let mut s2 = state.fork();
            s2.create_object(PlayerId(0), Zone::Battlefield, 999)
        };
        group.bench_with_input(BenchmarkId::from_parameter(size), &state, |b, st| {
            b.iter(|| {
                let mut child = st.fork();
                if let Some(o) = child.get_mut(target) {
                    o.tap();
                    o.add_counter(CounterKind::PlusOnePlusOne, 1);
                }
                criterion::black_box(child);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fork, bench_fork_then_mutate);
criterion_main!(benches);
