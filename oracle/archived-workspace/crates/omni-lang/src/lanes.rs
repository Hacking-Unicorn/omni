//! Adaptive parallel lane runner for workspace selftests and differential suites.
//!
//! Mirrors `omni_selftest_lane_count` / `run_selftest_tasks_in_lanes` in
//! `omni_rust_bootstrap.rs`:
//! - `OMNI_SELFTEST_LANES=N` overrides explicitly (clamped >= 1).
//! - Default leaves 1 core free for OS responsiveness:
//!   `max(1, available_parallelism - 1)`.
//! - Fixed static round-robin partition and fixed-order aggregation guarantee
//!   byte-identical output and identical check counts on any lane count.
//! - Nested lane calls run sequentially on worker threads to avoid
//!   oversubscription.
//! - Graceful fallback to sequential execution if thread spawning fails.

use std::cell::Cell;
use std::sync::Arc;

thread_local! {
    static IN_SELFTEST_LANE: Cell<bool> = const { Cell::new(false) };
}

/// Returns the adaptive parallel lane count for selftest runners:
///   - `OMNI_SELFTEST_LANES=N` overrides explicitly (clamped >= 1).
///   - Default leaves 1 core free for OS responsiveness: `max(1, available_parallelism - 1)`.
pub fn omni_selftest_lane_count() -> usize {
    if let Ok(raw) = std::env::var("OMNI_SELFTEST_LANES") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            return n.max(1);
        }
    }
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    cores.saturating_sub(1).max(1)
}

pub type SelftestLaneTask = (&'static str, fn() -> Result<usize, String>);

/// Executes independent selftest tasks across adaptive parallel lanes using a fixed static
/// round-robin partition and aggregates results in fixed check order (`0..tasks.len()`).
pub fn run_selftest_tasks_in_lanes(tasks: &[SelftestLaneTask]) -> Result<usize, String> {
    let n_tasks = tasks.len();
    if n_tasks == 0 {
        return Ok(0);
    }
    let in_lane = IN_SELFTEST_LANE.with(|c| c.get());
    let lanes = if in_lane {
        1
    } else {
        omni_selftest_lane_count().min(n_tasks)
    };
    if lanes <= 1 {
        let mut total = 0usize;
        for &(_name, f) in tasks {
            total += f()?;
        }
        return Ok(total);
    }
    let mut queues: Vec<Vec<(usize, SelftestLaneTask)>> = (0..lanes).map(|_| Vec::new()).collect();
    for (idx, &task) in tasks.iter().enumerate() {
        queues[idx % lanes].push((idx, task));
    }
    let mut handles: Vec<std::thread::JoinHandle<Vec<(usize, Result<usize, String>)>>> =
        Vec::with_capacity(lanes);
    let mut fallback: Vec<(usize, Result<usize, String>)> = Vec::new();
    for queue in queues {
        let q_worker = queue.clone();
        match std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                IN_SELFTEST_LANE.with(|c| c.set(true));
                let mut out = Vec::with_capacity(q_worker.len());
                for (idx, (_name, f)) in q_worker {
                    out.push((idx, f()));
                }
                out
            }) {
            Ok(h) => handles.push(h),
            Err(_) => {
                // Graceful fallback if threads are unavailable on this host.
                for (idx, (_name, f)) in queue {
                    fallback.push((idx, f()));
                }
            }
        }
    }
    let mut ordered: Vec<Option<Result<usize, String>>> = (0..n_tasks).map(|_| None).collect();
    for (idx, res) in fallback {
        ordered[idx] = Some(res);
    }
    for h in handles {
        let lane_out = h
            .join()
            .map_err(|_| String::from("Selftest worker lane panicked"))?;
        for (idx, res) in lane_out {
            ordered[idx] = Some(res);
        }
    }
    let mut total = 0usize;
    for (idx, slot) in ordered.into_iter().enumerate() {
        let res = slot.ok_or_else(|| format!("Selftest task #{} missing result", idx))?;
        total += res?;
    }
    Ok(total)
}

/// Executes independent corpus items across adaptive parallel lanes using a fixed static
/// round-robin partition and returns results in fixed corpus order (`0..items.len()`).
pub fn run_selftest_items_in_lanes<T, R, F>(items: Vec<T>, f: F) -> Result<Vec<R>, String>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> Result<R, String> + Send + Sync + 'static,
{
    let n_items = items.len();
    if n_items == 0 {
        return Ok(Vec::new());
    }
    let in_lane = IN_SELFTEST_LANE.with(|c| c.get());
    let lanes = if in_lane {
        1
    } else {
        omni_selftest_lane_count().min(n_items)
    };
    if lanes <= 1 {
        let mut out = Vec::with_capacity(n_items);
        for item in items {
            out.push(f(item)?);
        }
        return Ok(out);
    }
    let mut queues: Vec<Vec<(usize, T)>> = (0..lanes).map(|_| Vec::new()).collect();
    for (idx, item) in items.into_iter().enumerate() {
        queues[idx % lanes].push((idx, item));
    }
    let f = Arc::new(f);
    let mut handles: Vec<std::thread::JoinHandle<Vec<(usize, Result<R, String>)>>> =
        Vec::with_capacity(lanes);
    let mut fallback: Vec<(usize, Result<R, String>)> = Vec::new();
    for queue in queues {
        let f_worker = Arc::clone(&f);
        // Move `queue` into the worker; if spawn fails, recover via fallback?
        // Note: Builder::spawn takes FnOnce so we only move on spawn; if spawn fails,
        // std::io::Error does not return the closure, so check spawn first on a single
        // probe only if needed — or better, spawn per-queue with a tiny wrapper?
        // Actually std::thread::Builder::spawn consumes the closure even on Err!
        // To guarantee zero-loss graceful fallback on spawn Err, use an Arc<Mutex<Option<Vec>>>!
        let slot = Arc::new(std::sync::Mutex::new(Some(queue)));
        let slot_worker = Arc::clone(&slot);
        match std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                IN_SELFTEST_LANE.with(|c| c.set(true));
                let q = slot_worker
                    .lock()
                    .expect("lane queue lock")
                    .take()
                    .unwrap_or_default();
                let mut out = Vec::with_capacity(q.len());
                for (idx, item) in q {
                    out.push((idx, f_worker(item)));
                }
                out
            }) {
            Ok(h) => handles.push(h),
            Err(_) => {
                // Graceful fallback if threads are unavailable on this host.
                let q = slot
                    .lock()
                    .expect("lane queue lock")
                    .take()
                    .unwrap_or_default();
                for (idx, item) in q {
                    fallback.push((idx, f(item)));
                }
            }
        }
    }
    let mut ordered: Vec<Option<Result<R, String>>> = (0..n_items).map(|_| None).collect();
    for (idx, res) in fallback {
        ordered[idx] = Some(res);
    }
    for h in handles {
        let lane_out = h
            .join()
            .map_err(|_| String::from("Selftest worker lane panicked"))?;
        for (idx, res) in lane_out {
            ordered[idx] = Some(res);
        }
    }
    let mut out = Vec::with_capacity(n_items);
    for (idx, slot) in ordered.into_iter().enumerate() {
        let res = slot.ok_or_else(|| format!("Selftest item #{} missing result", idx))?;
        out.push(res?);
    }
    Ok(out)
}
