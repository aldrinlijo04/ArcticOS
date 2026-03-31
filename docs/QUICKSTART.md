# ARTICOS Quick Start Guide

Get up and running with ARTICOS GPU scheduler simulation in 5 minutes.

## Installation

1. **Install Rust (if not already installed)**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default nightly
   ```

2. **Clone and Build**
   ```bash
   cd ArcticOS
   cargo build --release
   ```

## Run Your First Simulation

```bash
cargo run --release --bin articos
```

This will run all 5 built-in scenarios demonstrating different scheduler behaviors.

## Understanding the Output

### Scenario 1: FIFO Basic
```
╔════════════════════════════════════════════════════════════╗
║           ARTICOS Scheduler Metrics Report                 ║
╠════════════════════════════════════════════════════════════╣
║ Total Tasks Processed:                                  10 ║
║ Average Wait Time:                               80.00 ms ║
║ Average Execution Time:                          100.00 ms ║
╚════════════════════════════════════════════════════════════╝
```

**Key Insight**: With 4 execution slots and 10 tasks, some tasks wait while others execute.

### Scenario 3: Priority Scheduler
```
Priority 10: 0.00ms avg wait   ← High priority, no wait!
Priority 5:  50.00ms avg wait  ← Medium priority
Priority 1:  100.00ms avg wait ← Low priority, longest wait
```

**Key Insight**: Priority scheduling reduces latency for important tasks but can delay others.

### Scenario 5: Resource Contention
```
2 SMs:  Avg wait 337.50ms, Total time 750ms
4 SMs:  Avg wait 150.00ms, Total time 375ms  ← 2x faster!
8 SMs:  Avg wait 60.00ms,  Total time 225ms  ← 3.3x faster!
```

**Key Insight**: More execution resources = lower wait times and higher throughput.

## Write Your Own Simulation

Create a new file `custom_sim.rs`:

```rust
use articos_scheduler::*;

fn main() {
    // Create a FIFO scheduler with 4 execution slots
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit 20 tasks
    for i in 0..20 {
        let task = Task::new(
            i,          // task ID
            0,          // priority (0 = low, 255 = high)
            1,          // resource requirement
            100,        // duration in milliseconds
        );
        executor.submit_task(task);
    }
    
    // Run simulation
    executor.run_until_complete(10000);
    
    // Print results
    executor.metrics().print_report();
}
```

## Common Tasks

### Run Tests
```bash
cargo test                    # All tests
cargo test -- --nocapture     # With output
cargo test integration        # Integration tests only
```

### Compare Schedulers
```rust
// FIFO: Fair, predictable
let fifo = Box::new(FifoScheduler::new());
let mut executor = Executor::new(4, fifo);

// Priority: Latency-optimized, potential starvation
let priority = Box::new(PriorityScheduler::new());
let mut executor = Executor::new(4, priority);
```

### Adjust Execution Resources
```rust
// Simulate low-end GPU (2 SMs)
let mut executor = Executor::new(2, scheduler);

// Simulate high-end GPU (108 SMs)
let mut executor = Executor::new(108, scheduler);
```

### Create Tasks with Priority
```rust
// High-priority real-time task
let critical = Task::new(1, 255, 1, 50);

// Medium-priority compute task
let compute = Task::new(2, 128, 1, 200);

// Low-priority background task
let background = Task::new(3, 10, 1, 100);
```

## GPU Analogy Reference

| ARTICOS Concept | GPU Hardware Equivalent |
|----------------|------------------------|
| `Task` | CUDA kernel launch |
| `Executor` | GPU with N SMs/CUs |
| `FifoScheduler` | Default CUDA stream |
| `PriorityScheduler` | CUDA high-priority stream |
| Execution slot | Streaming Multiprocessor |
| Wait time | Queue latency |
| Starvation | Priority inversion |

## Troubleshooting

### "command not found: cargo"
Install Rust: https://rustup.rs

### Tests failing
Ensure you're using nightly Rust:
```bash
rustup default nightly
```

### Simulation too fast/slow
Adjust task durations:
```rust
Task::new(id, priority, 1, 10);    // Fast (10ms)
Task::new(id, priority, 1, 1000);  // Slow (1s)
```

## Next Steps

1. **Read the full README** - [README.md](../README.md)
2. **Understand GPU concepts** - [docs/GPU_CONCEPTS.md](GPU_CONCEPTS.md)
3. **Explore the code** - Start with `scheduler/src/lib.rs`
4. **Implement custom scheduler** - See README for `Scheduler` trait
5. **Experiment with scenarios** - Modify `control-plane/src/main.rs`

## Example Experiments

### Experiment 1: Starvation Analysis
```rust
// Create priority scheduler with 2 slots
let scheduler = Box::new(PriorityScheduler::new());
let mut executor = Executor::new(2, scheduler);

// 5 low-priority tasks
for i in 0..5 {
    executor.submit_task(Task::new(i, 1, 1, 50));
}

// 20 high-priority tasks
for i in 5..25 {
    executor.submit_task(Task::new(i, 10, 1, 50));
}

executor.run_until_complete(50000);
let agg = executor.metrics().aggregate();

println!("Starved tasks: {}", agg.starved_tasks);
println!("Max wait time: {}ms", agg.max_wait_time_ms);
```

### Experiment 2: Throughput Analysis
```rust
use std::time::Instant;

let start = Instant::now();
// ... run simulation ...
let duration = start.elapsed();

let throughput = num_tasks as f64 / duration.as_secs_f64();
println!("Throughput: {:.2} tasks/sec", throughput);
```

## Getting Help

- **Issues**: Found a bug? Open an issue on GitHub
- **Questions**: Check docs/ folder for in-depth explanations
- **Examples**: See `control-plane/src/main.rs` for 5 complete scenarios

## Performance Tips

1. **Use release builds** for accurate timing: `cargo run --release`
2. **Adjust simulation duration** based on task count
3. **Profile with tracing** - set `RUST_LOG=debug` for detailed logs
4. **Batch submissions** for large-scale simulations

---

**Ready to explore GPU scheduling? Start with `cargo run --release --bin articos`!** 🚀
