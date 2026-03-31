# ARCTICOS Phase 1: GPU Runtime Scheduler Simulation

A production-structured, CPU-based simulation of a GPU runtime scheduler built in Rust. ARCTICOS (Arctic Operating System) is a GPU-first systems runtime, and this Phase 1 implementation models the core scheduling behaviors without requiring actual GPU hardware.

## 🎯 Project Goals

- **Model GPU scheduling behavior** - Simulate how GPU hardware schedulers (like NVIDIA's SM scheduler) manage concurrent workloads
- **Production-ready structure** - Clean separation of concerns, modular design, comprehensive testing
- **Educational foundation** - Clear comments explaining GPU-equivalent behavior for each component
- **Metrics-driven** - Built-in performance measurement and starvation detection

## 🏗️ Architecture

```
ArcticOS/
├── scheduler/           # Core scheduling library
│   ├── src/
│   │   ├── task.rs     # Task definition (analogous to kernel launch)
│   │   ├── scheduler.rs # Scheduler trait
│   │   ├── fifo.rs     # FIFO scheduler implementation
│   │   ├── priority.rs # Priority-based scheduler
│   │   ├── executor.rs # Simulated SM execution slots
│   │   └── metrics.rs  # Performance metrics collection
│   └── tests/
│       └── integration_tests.rs
├── control-plane/       # Control plane that submits tasks
│   └── src/
│       └── main.rs     # Simulation scenarios
└── Cargo.toml          # Workspace configuration
```

## 📦 Components

### Task (`task.rs`)
Represents a schedulable workload unit with:
- **ID**: Unique task identifier
- **Priority**: 0-255 (higher = more important)
- **Resource requirement**: Simulated compute units needed
- **Duration**: Expected execution time in milliseconds

**GPU Analogy**: Similar to a CUDA kernel launch or compute dispatch in Vulkan/Metal.

### Scheduler Trait (`scheduler.rs`)
Core interface for scheduling policies:
- `submit(task)` - Add task to queue
- `next()` - Get next task to execute
- `has_pending()` - Check for queued tasks

**GPU Analogy**: Software interface to GPU command queue management.

### FIFO Scheduler (`fifo.rs`)
First-In-First-Out scheduling:
- ✅ **Fair**: No starvation, every task eventually runs
- ✅ **Predictable**: Execution order matches submission order
- ❌ **Non-optimal**: Doesn't consider priority or optimize throughput

**GPU Analogy**: Default CUDA stream behavior.

### Priority Scheduler (`priority.rs`)
Priority-based scheduling with FIFO within priority levels:
- ✅ **Priority-aware**: High-priority tasks execute first
- ✅ **Low latency for critical work**: Important tasks run quickly
- ⚠️ **Starvation possible**: Low-priority tasks may wait indefinitely

**GPU Analogy**: CUDA stream priorities (`cudaStreamCreateWithPriority`).

### Executor (`executor.rs`)
Simulates GPU execution resources:
- Fixed number of **execution slots** (simulated SMs)
- **No preemption** - tasks run to completion
- Parallel execution when slots available
- Metrics collection for each task

**GPU Analogy**: Models NVIDIA Streaming Multiprocessors (SMs) or AMD Compute Units (CUs).

### Metrics (`metrics.rs`)
Performance analysis and starvation detection:
- Wait time, execution time, total time per task
- Aggregate statistics (average, min, max)
- Starvation detection (wait time > 10× average execution)
- Priority-level analysis

**GPU Analogy**: Similar to NVIDIA Nsight, AMD rocProf, or Intel VTune profilers.

## 🚀 Getting Started

### Prerequisites
- Rust (nightly) - `rustup default nightly`
- No GPU required for Phase 1

### Build
```bash
cd ArcticOS
cargo build --release
```

### Run Simulation
```bash
cargo run --release --bin articos
```

### Run Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run only integration tests
cargo test --test integration_tests
```

## 📊 Simulation Scenarios

The control-plane binary runs five scenarios demonstrating different scheduling behaviors:

### Scenario 1: FIFO Basic
- 10 uniform tasks, equal priority
- Demonstrates fair, predictable scheduling
- GPU equivalent: Sequential kernel launches on default stream

### Scenario 2: FIFO Varying Durations
- Mixed short and long tasks
- Shows head-of-line blocking effect
- GPU equivalent: Mix of compute-heavy and lightweight kernels

### Scenario 3: Priority Basic
- Tasks with high, medium, and low priorities
- Demonstrates priority scheduling benefits
- GPU equivalent: CUDA high-priority streams

### Scenario 4: Priority Starvation
- Continuous high-priority tasks + few low-priority
- Demonstrates starvation scenario
- GPU equivalent: Real-time rendering starving background compute

### Scenario 5: Resource Contention
- Compares 2, 4, and 8 simulated SMs
- Shows impact of execution resources on throughput
- GPU equivalent: Comparing GPUs with different SM counts

## 🧪 Sample Output

```
╔════════════════════════════════════════════════════════════╗
║           ARTICOS Scheduler Metrics Report                 ║
╠════════════════════════════════════════════════════════════╣
║ Total Tasks Processed:                                  10 ║
║ Simulation Time:                                    357 ms ║
╠════════════════════════════════════════════════════════════╣
║ Average Wait Time:                                45.30 ms ║
║ Average Execution Time:                          100.20 ms ║
║ Average Total Time:                              145.50 ms ║
╠════════════════════════════════════════════════════════════╣
║ Min Wait Time:                                        0 ms ║
║ Max Wait Time:                                      152 ms ║
╠════════════════════════════════════════════════════════════╣
║ Starved Tasks:                                           0 ║
╚════════════════════════════════════════════════════════════╝
```

## 🧩 Adding Custom Schedulers

To add a new scheduler, implement the `Scheduler` trait:

```rust
use articos_scheduler::*;

pub struct MyScheduler {
    // Your state here
}

impl Scheduler for MyScheduler {
    fn submit(&mut self, task: Task) {
        // Add task to your queue
    }
    
    fn next(&mut self) -> Option<Task> {
        // Return next task to execute
    }
    
    fn has_pending(&self) -> bool {
        // Check if tasks remain
    }
    
    fn pending_count(&self) -> usize {
        // Return queue size
    }
    
    fn name(&self) -> &str {
        "MyScheduler"
    }
}
```

## 📈 Key Metrics

- **Wait Time**: Time spent in scheduler queue before execution
- **Execution Time**: Actual task runtime (simulated)
- **Total Time**: Wait + Execution
- **Starvation**: Detected when wait time > 10× average execution time
- **Throughput**: Tasks completed per unit time
- **Fairness**: Variance in wait times (lower = more fair)

## 🔬 GPU Analogies

| Component | GPU Hardware Equivalent |
|-----------|------------------------|
| Task | Kernel launch / Command buffer |
| Scheduler | GPU command processor |
| Execution Slot | Streaming Multiprocessor (SM) / Compute Unit (CU) |
| Priority | Stream priority / Queue priority |
| Duration | Kernel execution time |
| No Preemption | SM wavefront execution model |
| Metrics | NVIDIA Nsight / AMD rocProf |

## 🛣️ Roadmap

### Phase 1 (Current): Scheduler Simulation ✅
- Task model with priority and resources
- FIFO and Priority schedulers
- Fixed SM slots, no preemption
- Comprehensive metrics

### Phase 2 (Future): Advanced Scheduling
- Shortest-Job-First (SJF) scheduler
- Multi-level feedback queues
- Dynamic resource allocation
- Preemption support

### Phase 3 (Future): GPU Integration
- CUDA/ROCm backend
- Real GPU scheduling
- Multi-GPU support
- Hardware profiling integration

## 🧪 Testing Strategy

- **Unit tests**: Individual component behavior (in each module)
- **Integration tests**: Full execution cycles across schedulers
- **Scenario tests**: Real-world simulation patterns
- **Starvation tests**: Verify detection mechanisms
- **Fairness tests**: Compare scheduler characteristics

## 📚 Related Concepts

- **GPU Scheduling**: How GPUs manage concurrent workloads
- **Stream/Queue Priorities**: CUDA streams, Vulkan queues, Metal command buffers
- **Starvation**: Priority inversion and unfairness issues
- **Throughput vs Latency**: Tradeoffs in scheduler design
- **Non-preemptive Execution**: Why GPUs traditionally don't preempt kernels

## 🤝 Contributing

This is a learning project for understanding GPU runtime systems. Contributions welcome!

Areas to explore:
- Additional scheduling algorithms (Round-robin, Weighted fair queuing)
- Better starvation mitigation strategies
- Dynamic priority adjustment
- Resource-aware scheduling (memory bandwidth, power)
- Visualization tools for schedule timelines

## 📄 License

MIT License - See LICENSE file

## 🙏 Acknowledgments

Inspired by:
- NVIDIA CUDA runtime scheduler
- AMD ROCm queue management
- Linux kernel scheduling algorithms
- Real-time operating system concepts

---

**Built with Rust 🦀 | GPU-First Design 🚀 | Production-Quality Code 💎**
