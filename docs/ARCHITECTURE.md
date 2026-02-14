# ARTICOS Project Architecture

## Directory Structure

```
ArcticOS/
├── Cargo.toml                    # Workspace configuration
├── README.md                     # Main documentation
├── LICENSE                       # MIT License
├── .gitignore                    # Git ignore rules
│
├── docs/                         # Documentation
│   ├── GPU_CONCEPTS.md          # GPU scheduling background
│   ├── QUICKSTART.md            # Quick start guide
│   └── ARCHITECTURE.md          # This file
│
├── scheduler/                    # Core scheduling library
│   ├── Cargo.toml               # Library dependencies
│   ├── src/
│   │   ├── lib.rs               # Public API exports
│   │   ├── task.rs              # Task definition
│   │   ├── scheduler.rs         # Scheduler trait
│   │   ├── fifo.rs              # FIFO implementation
│   │   ├── priority.rs          # Priority implementation
│   │   ├── executor.rs          # Execution engine
│   │   └── metrics.rs           # Performance metrics
│   └── tests/
│       └── integration_tests.rs # Integration tests
│
└── control-plane/                # Task submission binary
    ├── Cargo.toml               # Binary dependencies
    └── src/
        └── main.rs              # Simulation scenarios
```

## Module Overview

### scheduler/src/lib.rs
- **Purpose**: Public API surface
- **Exports**: All public types and traits
- **Usage**: `use articos_scheduler::*;`

### scheduler/src/task.rs
- **Core Type**: `Task` struct
- **Fields**: `id`, `priority`, `resource_requirement`, `duration_ms`
- **Helper Functions**: Time utilities
- **Tests**: Task creation and validation

### scheduler/src/scheduler.rs
- **Core Trait**: `Scheduler`
- **Methods**: `submit()`, `next()`, `has_pending()`, `pending_count()`
- **Purpose**: Abstract scheduling policy interface

### scheduler/src/fifo.rs
- **Implementation**: `FifoScheduler`
- **Data Structure**: `VecDeque<Task>`
- **Complexity**: O(1) enqueue/dequeue
- **Tests**: Order preservation, priority independence

### scheduler/src/priority.rs
- **Implementation**: `PriorityScheduler`
- **Data Structure**: `BinaryHeap<PriorityTask>`
- **Complexity**: O(log n) enqueue/dequeue
- **Features**: Priority ordering, FIFO within priority
- **Tests**: Priority ordering, distribution tracking

### scheduler/src/executor.rs
- **Core Type**: `Executor`
- **Components**:
  - `ExecutionSlot`: Simulated SM
  - Scheduler instance
  - Metrics collector
- **Responsibilities**:
  - Schedule tasks to slots
  - Detect task completion
  - Collect metrics
- **Tests**: Slot lifecycle, execution flow

### scheduler/src/metrics.rs
- **Core Types**:
  - `TaskMetrics`: Per-task measurements
  - `AggregateMetrics`: Summary statistics
  - `MetricsCollector`: Collection and analysis
- **Measurements**:
  - Wait time, execution time, total time
  - Min/max/average statistics
  - Starvation detection
  - Priority-level analysis
- **Tests**: Calculation accuracy

### control-plane/src/main.rs
- **Purpose**: Demonstrate scheduler capabilities
- **Scenarios**:
  1. FIFO Basic - Uniform tasks
  2. FIFO Varying - Mixed durations
  3. Priority Basic - Mixed priorities
  4. Priority Starvation - Resource starvation
  5. Resource Contention - SM count comparison
- **Output**: Formatted metrics reports

## Data Flow

```
┌─────────────────┐
│  Control Plane  │ ← Creates and submits tasks
└────────┬────────┘
         │
         │ submit_task()
         ▼
┌─────────────────┐
│    Executor     │ ← Manages execution
│  ┌──────────┐   │
│  │Scheduler │   │ ← Decides task order
│  └──────────┘   │
│  ┌──────────┐   │
│  │  Slots   │   │ ← Execute tasks
│  └──────────┘   │
│  ┌──────────┐   │
│  │ Metrics  │   │ ← Collect performance data
│  └──────────┘   │
└─────────────────┘
```

## Task Lifecycle

```
1. Creation
   Task::new(id, priority, resources, duration)
   ↓
2. Submission
   executor.submit_task(task) → scheduler.submit(task)
   ↓
3. Queuing
   Task waits in scheduler queue
   ↓
4. Scheduling
   scheduler.next() → returns task when ready
   ↓
5. Execution
   Task assigned to available ExecutionSlot
   ↓
6. Completion
   Task duration elapsed, slot freed
   ↓
7. Metrics Recording
   metrics.record_task(task, started_at, completed_at)
```

## Scheduling Cycle

```rust
loop {
    // 1. Check for completed tasks
    for slot in slots {
        if task_complete(slot) {
            free_slot(slot);
            record_metrics(task);
        }
    }
    
    // 2. Schedule new tasks
    for slot in slots {
        if slot.is_available() {
            if let Some(task) = scheduler.next() {
                slot.assign_task(task);
            }
        }
    }
    
    // 3. Check termination
    if all_done() { break; }
    
    // 4. Simulate time passage
    sleep(100us);
}
```

## Design Decisions

### Why Trait-Based Scheduler?
- **Extensibility**: Easy to add new scheduling algorithms
- **Testability**: Mock schedulers for testing
- **Type Safety**: Compile-time guarantees
- **Flexibility**: Runtime scheduler selection

### Why No Preemption?
- **GPU Reality**: Real GPU hardware doesn't preempt warps
- **Simplicity**: Phase 1 focuses on core concepts
- **Future**: Phase 2 may add preemption for comparison

### Why CPU Simulation?
- **Accessibility**: No GPU required for development
- **Debugging**: Easier to debug than GPU code
- **Portability**: Runs on any platform
- **Speed**: Faster iteration than real GPU testing

### Why Rust?
- **Safety**: Memory safety without garbage collection
- **Performance**: Zero-cost abstractions
- **Concurrency**: Fearless concurrency primitives
- **Tooling**: Excellent toolchain (cargo, clippy, rustfmt)

## Testing Strategy

### Unit Tests
- Located in each module (`#[cfg(test)] mod tests`)
- Test individual components in isolation
- Fast execution, no external dependencies

### Integration Tests
- Located in `scheduler/tests/`
- Test full execution cycles
- Verify scheduler + executor + metrics interaction

### Documentation Tests
- Embedded in doc comments
- Ensure examples stay up-to-date
- Run with `cargo test`

## Extension Points

### Adding a New Scheduler

1. **Create new file**: `scheduler/src/my_scheduler.rs`
2. **Implement trait**:
   ```rust
   pub struct MyScheduler { /* ... */ }
   impl Scheduler for MyScheduler { /* ... */ }
   ```
3. **Add to lib.rs**:
   ```rust
   pub mod my_scheduler;
   pub use my_scheduler::MyScheduler;
   ```
4. **Write tests**: Unit + integration tests
5. **Use in control-plane**: Add scenario

### Adding New Metrics

1. **Extend `TaskMetrics`** or `AggregateMetrics`
2. **Update `MetricsCollector::record_task()`**
3. **Update `MetricsCollector::aggregate()`**
4. **Update `print_report()` for display**

### Adding Resource Constraints

1. **Extend `Task`** with resource fields
2. **Update `ExecutionSlot`** to track resource usage
3. **Modify `Executor::schedule_cycle()`** for resource checks
4. **Add resource exhaustion tests**

## Performance Considerations

### Scheduler Performance
- **FIFO**: O(1) enqueue/dequeue via `VecDeque`
- **Priority**: O(log n) enqueue/dequeue via `BinaryHeap`
- **Memory**: Linear in number of pending tasks

### Executor Performance
- **Scheduling Cycle**: O(slots + scheduled_tasks)
- **Completion Check**: O(slots)
- **Memory**: Fixed overhead per slot

### Simulation Accuracy
- **Time Resolution**: Nanosecond timestamps
- **Sleep Granularity**: 100 microsecond cycles
- **Duration Accuracy**: ±10ms variance expected

## Future Roadmap

### Phase 2: Advanced Scheduling
- [ ] Shortest-Job-First (SJF) scheduler
- [ ] Round-robin with time slicing
- [ ] Multi-level feedback queues
- [ ] Dynamic resource allocation
- [ ] Preemption support
- [ ] Work-stealing between slots

### Phase 3: GPU Integration
- [ ] CUDA backend for real GPU execution
- [ ] ROCm/HIP support for AMD GPUs
- [ ] Vulkan compute queue integration
- [ ] Multi-GPU scheduling
- [ ] Hardware profiling integration
- [ ] Comparison: simulated vs actual performance

### Potential Enhancements
- [ ] Web-based visualization dashboard
- [ ] Timeline view of task execution
- [ ] Real-time metrics streaming
- [ ] Configuration files for scenarios
- [ ] Benchmarking suite
- [ ] Scheduler recommendation engine

## References

- **CUDA Programming Guide**: NVIDIA's official documentation
- **ROCm Documentation**: AMD's GPU compute platform
- **Vulkan Spec**: Cross-platform GPU API
- **Operating Systems: Three Easy Pieces**: Scheduling algorithms
- **"GPU Computing Gems"**: GPU scheduling optimization

---

**For questions about architecture decisions, see the main README or docs/GPU_CONCEPTS.md**
