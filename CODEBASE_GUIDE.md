# ARTICOS Codebase Guide

A complete explanation of the ARTICOS Phase 1 GPU runtime scheduler simulator codebase.

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Architecture & Design](#architecture--design)
4. [Module Breakdown](#module-breakdown)
5. [Data Flow](#data-flow)
6. [Key Abstractions](#key-abstractions)
7. [Usage Examples](#usage-examples)
8. [Integration Points](#integration-points)
9. [Testing Strategy](#testing-strategy)
10. [Extension Guide](#extension-guide)

---

## Overview

**ARTICOS** (Arctic Operating System) is a GPU-first systems runtime. **ARTICOS Phase 1** implements a CPU-based simulation of a GPU runtime scheduler that models:

- **Task Management**: Workload representation with priority and resource requirements
- **Scheduling Policies**: FIFO and Priority-based scheduling algorithms
- **GPU Resource Simulation**: Fixed number of execution slots (simulated Streaming Multiprocessors)
- **Performance Metrics**: Comprehensive measurement including wait time, starvation detection
- **No Preemption Model**: Tasks run to completion, matching real GPU behavior

### Why This Approach?

Real GPUs are complex and expensive. ARTICOS Phase 1 lets you:
- ✅ Understand GPU scheduling without hardware
- ✅ Compare different scheduling policies efficiently
- ✅ Develop scheduling algorithms before GPU integration
- ✅ Benchmark performance characteristics
- ✅ Detect fairness and starvation issues

---

## Project Structure

```
ArcticOS/
├── Cargo.toml                    # Workspace root configuration
├── README.md                     # Main project documentation
├── SUMMARY.md                    # Verification & delivery report
├── CODEBASE_GUIDE.md            # This file
│
├── scheduler/                    # Core scheduler library
│   ├── Cargo.toml               # Library manifest
│   ├── src/
│   │   ├── lib.rs               # Library root, module exports
│   │   ├── task.rs              # Task definition & types
│   │   ├── scheduler.rs         # Scheduler trait
│   │   ├── executor.rs          # Executor engine (main runtime)
│   │   ├── fifo.rs              # FIFO scheduler implementation
│   │   ├── priority.rs          # Priority scheduler implementation
│   │   └── metrics.rs           # Performance metrics collection
│   └── tests/
│       └── integration_tests.rs  # End-to-end test scenarios
│
├── control-plane/                # Simulation binary (CLI)
│   ├── Cargo.toml               # Binary manifest
│   └── src/
│       └── main.rs              # 5 simulation scenarios
│
└── docs/
    ├── GPU_CONCEPTS.md          # GPU hardware & software explanation
    ├── ARCHITECTURE.md          # Detailed architecture reference
    └── QUICKSTART.md            # Getting started guide
```

### Crate Organization

**Two-Crate Workspace**:
1. **`articos-scheduler`**: Library providing core simulation (reusable)
2. **`articos-control-plane`**: Binary demonstrating simulation scenarios

This separation allows:
- Reusability (library can be used in other projects)
- Clear API boundaries
- Testability (test library independently)
- Future integration with real GPU code

---

## Architecture & Design

### High-Level Architecture Diagram

```
Control Plane (Binary)
    ↓
    ├── Submit Task 1 ──┐
    ├── Submit Task 2 ──┤
    ├── Submit Task N ──┤
                        ↓
                  Scheduler (Trait)
                  - FIFO Implementation
                  - Priority Implementation
                        ↓
                  Executor (Runtime)
                  ├── Execution Slot 0 (SM 0)
                  ├── Execution Slot 1 (SM 1)
                  ├── Execution Slot 2 (SM 2)
                  └── Execution Slot 3 (SM 3)
                        ↓
                  Metrics Collector
                  - Per-task metrics
                  - Starvation detection
                  - Aggregate statistics
```

### Core Design Patterns

#### 1. **Trait-Based Abstraction** (Scheduler)
```rust
pub trait Scheduler: Send + Sync {
    fn submit(&mut self, task: Task);
    fn next(&mut self) -> Option<Task>;
    fn has_pending(&self) -> bool;
    fn pending_count(&self) -> usize;
    fn name(&self) -> &str;
}
```

**Benefits**:
- Multiple scheduler implementations (FIFO, Priority, future: SJF, etc.)
- Easy testing with mock schedulers
- Plugin architecture for custom policies
- Compiler enforces interface consistency

#### 2. **Composition Over Inheritance** (Executor)
```rust
pub struct Executor {
    slots: Vec<ExecutionSlot>,
    scheduler: Box<dyn Scheduler>,  // Dependency injection
    metrics: MetricsCollector,
}
```

**Benefits**:
- Runtime scheduler selection
- Flexible composition
- Clear responsibility separation
- Easy to swap implementations

#### 3. **Type Aliases for Clarity**
```rust
pub type TaskId = u64;
pub type Priority = u8;
pub type ResourceRequirement = u32;
pub type DurationMs = u64;
```

**Benefits**:
- Self-documenting code
- Prevents type confusion (id vs priority)
- Easy to change underlying types
- Clear intent at call sites

---

## Module Breakdown

### 1. **task.rs** - Task Definition

**Purpose**: Defines the fundamental unit of work in the scheduler

**Key Types**:

```rust
pub struct Task {
    pub id: TaskId,                    // Unique identifier (0..N)
    pub priority: Priority,            // 0-255, higher = more urgent
    pub resource_requirement: u32,     // Compute units required
    pub duration_ms: u64,              // Expected execution time
    pub submitted_at: u128,            // When task was created (nanoseconds)
}
```

**Key Functions**:

| Function | Purpose | GPU Analogy |
|----------|---------|-------------|
| `Task::new(...)` | Create task with all parameters | Create kernel launch |
| `Task::with_duration(...)` | Create task with defaults | Quick kernel launch wrapper |
| `current_time_nanos()` | Get current timestamp | GPU clock reading |
| `nanos_to_millis(...)` | Convert timescale | Time unit conversion |

**GPU Analogy**:
- Task ≈ Kernel launch (CUDA) / Compute dispatch (Vulkan)
- Priority ≈ Stream priority (CUDA) / Queue priority (Vulkan)
- Duration ≈ Estimated kernel runtime
- ID ≈ Kernel name or grid dimension

**Example**:
```rust
// Create a medium-priority task
let task = Task::new(
    42,                             // Task ID
    5,                              // Priority (0-255)
    2,                              // Resource requirement
    100                             // Duration in milliseconds
);

// Or quick version
let quick = Task::with_duration(1, 50);  // ID=1, 50ms duration
```

### 2. **scheduler.rs** - Scheduler Trait

**Purpose**: Defines the interface for scheduling policies

**Core Trait**:

```rust
pub trait Scheduler: Send + Sync {
    fn submit(&mut self, task: Task);        // Add to queue
    fn next(&mut self) -> Option<Task>;      // Get next to execute
    fn has_pending(&self) -> bool;           // Any tasks left?
    fn pending_count(&self) -> usize;        // How many waiting?
    fn name(&self) -> &str;                  // For logging
}
```

**Key Enum**:

```rust
pub enum ScheduleResult {
    Scheduled,     // Task successfully started
    NoSlots,       // No execution slots available
    NoTasks,       // No tasks in queue
}
```

**Usage Pattern**:
```rust
let mut scheduler: Box<dyn Scheduler> = Box::new(FifoScheduler::new());
scheduler.submit(task1);
scheduler.submit(task2);

while let Some(next_task) = scheduler.next() {
    // Execute next_task
}
```

**Why a Trait?**
- Pluggable implementations (FIFO, Priority, custom)
- Testing with mock schedulers
- Runtime strategy selection
- Future extensibility

### 3. **fifo.rs** - FIFO Scheduler Implementation

**Purpose**: Implements First-In-First-Out scheduling (simplest, fairest)

**Key Type**:

```rust
pub struct FifoScheduler {
    queue: VecDeque<Task>,  // O(1) enqueue/dequeue
}
```

**Characteristics**:

| Aspect | Behavior |
|--------|----------|
| Order | Submission order |
| Fairness | Perfect (all tasks eventually execute) |
| Latency | Variable (depends on queue size) |
| Complexity | O(1) per operation |
| Starvation | None (FIFO prevents starvation) |

**Algorithm**:

```
submit(task):
    queue.push_back(task)

next():
    return queue.pop_front()
```

**GPU Analogy**:
- Default CUDA stream behavior
- Simple command queue processing
- Used when all tasks are equal importance

**When to Use**:
- ✅ Fair resource sharing needed
- ✅ All tasks have equal importance
- ✅ Simplicity preferred over optimization
- ❌ Latency-sensitive tasks
- ❌ Mixed-priority workloads

**Example**:
```rust
let scheduler = Box::new(FifoScheduler::new());
let mut executor = Executor::new(4, scheduler);  // 4 SMs

// Tasks execute in submission order
executor.submit_task(Task::with_duration(1, 100));
executor.submit_task(Task::with_duration(2, 50));
executor.submit_task(Task::with_duration(3, 75));
// Execution order: Task 1, Task 2, Task 3
```

### 4. **priority.rs** - Priority Scheduler Implementation

**Purpose**: Implements priority-based scheduling with FIFO within priority levels

**Key Types**:

```rust
pub struct PriorityScheduler {
    heap: BinaryHeap<PriorityTask>,  // Max-heap for priority ordering
    priority_counts: [usize; 256],   // Track count per priority level
}

// Internal wrapper for ordering
struct PriorityTask {
    task: Task,
}
```

**Ordering Logic**:

```
Compare two tasks:
1. Higher priority first (255 > 254 > ... > 0)
2. If same priority: earlier submission first (FIFO)
3. If same timestamp (unlikely): lower ID first (tiebreaker)
```

**Characteristics**:

| Aspect | Behavior |
|--------|----------|
| Order | Priority DESC, then submission ASC |
| Fairness | Possible starvation of low-priority |
| Latency | Low for high-priority tasks |
| Complexity | O(log n) per operation |
| Starvation | Yes (high-priority can starve low-priority) |

**Algorithm**:

```
submit(task):
    heap.push(PriorityTask { task })
    priority_counts[task.priority] += 1

next():
    if let Some(pt) = heap.pop():
        priority_counts[pt.task.priority] -= 1
        return Some(pt.task)
    return None
```

**GPU Analogy**:
- CUDA stream priorities (`cudaStreamCreateWithPriority`)
- ROCm queue priorities
- Vulkan queue priorities
- Real-time rendering at high priority, background at low

**When to Use**:
- ✅ Mixed-priority workloads
- ✅ Critical tasks need low latency
- ✅ Background compute acceptable to delay
- ⚠️ Must monitor for starvation
- ⚠️ Needs fairness mitigation (aging, quotas)

**Example**:
```rust
let scheduler = Box::new(PriorityScheduler::new());
let mut executor = Executor::new(4, scheduler);

executor.submit_task(Task::new(1, 1, 1, 100));      // Low priority
executor.submit_task(Task::new(2, 255, 1, 100));    // High priority
executor.submit_task(Task::new(3, 5, 1, 100));      // Medium priority

// Execution order: Task 2 (255), Task 3 (5), Task 1 (1)
// Within same priority: FIFO order preserved
```

**Priority Distribution Tracking**:
```rust
let dist = scheduler.priority_distribution();
// Returns Vec<(Priority, Count)> for analysis
// Useful for detecting starvation / load balancing
```

### 5. **executor.rs** - Execution Engine

**Purpose**: Runtime that manages execution slots and coordinates scheduling

**Key Types**:

```rust
struct ExecutionSlot {
    id: usize,
    current_task: Option<Task>,
    started_at: Option<u128>,
    complete_at: Option<u128>,
}

pub struct Executor {
    slots: Vec<ExecutionSlot>,              // Simulated SMs
    scheduler: Box<dyn Scheduler>,
    metrics: MetricsCollector,
    completed_tasks: usize,
}
```

**Key Operations**:

| Method | Purpose |
|--------|---------|
| `Executor::new(slots, scheduler)` | Create executor with N slots |
| `submit_task(task)` | Add task to scheduler queue |
| `schedule_cycle()` | Run one scheduling iteration |
| `run_until_complete(max_iters)` | Run until all tasks done |
| `run_for_duration(ms)` | Run for fixed duration |
| `metrics()` | Get performance metrics |

**Execution Slot Lifecycle**:

```
[Available] 
    ↓ (assign_task)
[Executing] (for task.duration_ms)
    ↓ (try_complete)
[Available]
```

**Scheduling Cycle Algorithm**:

```rust
fn schedule_cycle():
    // Step 1: Check for task completions
    for each slot:
        if task_complete:
            record_metrics(task)
            free_slot()
    
    // Step 2: Schedule new tasks
    for each available_slot:
        if scheduler.has_pending():
            task = scheduler.next()
            assign_to_slot(task)
            schedule++
    
    return schedule_count
```

**GPU Analogy**:
- SMs (Streaming Multiprocessors) ≈ ExecutionSlots
- Fixed number in hardware ≈ slots.len()
- Warp scheduler ≈ schedule_cycle()
- Kernel execution ≈ task in slot

**Resource Management**:

```rust
pub fn available_slots(&self) -> usize
pub fn executing_count(&self) -> usize
pub fn pending_count(&self) -> usize
pub fn total_slots(&self) -> usize
```

**Example - Custom Simulation**:
```rust
let scheduler = Box::new(FifoScheduler::new());
let mut executor = Executor::new(8, scheduler);  // 8 SMs

// Submit many tasks
for i in 0..100 {
    executor.submit_task(Task::with_duration(i, 50));
}

// Run simulation
executor.run_until_complete(50000);

// Analyze results
let metrics = executor.metrics();
metrics.print_report();
println!("Tasks completed: {}", executor.completed_count());
```

### 6. **metrics.rs** - Performance Metrics

**Purpose**: Collect and analyze performance data (wait time, starvation, etc.)

**Key Types**:

```rust
pub struct TaskMetrics {
    pub task_id: TaskId,
    pub wait_time_ms: u128,           // Queue to execution
    pub execution_time_ms: u128,      // Actual runtime
    pub total_time_ms: u128,          // Submit to complete
    pub submitted_at: u128,
    pub started_at: u128,
    pub completed_at: u128,
    pub priority: u8,
}

pub struct AggregateMetrics {
    pub total_tasks: usize,
    pub avg_wait_time_ms: f64,
    pub avg_execution_time_ms: f64,
    pub avg_total_time_ms: f64,
    pub max_wait_time_ms: u128,
    pub min_wait_time_ms: u128,
    pub starved_tasks: usize,  // wait > 10x avg_execution
    pub total_simulation_time_ms: u128,
}

pub struct MetricsCollector {
    task_metrics: Vec<TaskMetrics>,
    simulation_start: u128,
}
```

**Starvation Detection**:

```rust
// A task is "starved" if:
starvation_threshold = avg_execution_time * 10.0
if task.wait_time_ms > starvation_threshold:
    starved_count++
```

**Metrics Analysis**:

```rust
// Get all per-task metrics
executor.metrics().task_metrics()

// Get aggregate statistics
let agg = executor.metrics().aggregate();
println!("Avg wait: {:.2}ms", agg.avg_wait_time_ms);
println!("Starved tasks: {}", agg.starved_tasks);

// Get tasks by priority
let by_priority = executor.metrics().tasks_by_priority();
for (priority, tasks) in by_priority {
    println!("Priority {}: {} tasks", priority, tasks.len());
}

// Pretty print report
executor.metrics().print_report();
```

**GPU Analogy**:
- Similar to NVIDIA Nsight profiler
- Similar to AMD rocProf profiling tool
- Equivalent to Intel VTune GPU metrics

**Example Output**:
```
╔════════════════════════════════════════════╗
║     ARTICOS Scheduler Metrics Report       ║
╠════════════════════════════════════════════╣
║ Total Tasks Processed:                 10 ║
║ Simulation Time:                   300 ms ║
╠════════════════════════════════════════════╣
║ Average Wait Time:                80.00 ms ║
║ Average Execution Time:          100.00 ms ║
║ Average Total Time:              180.00 ms ║
╠════════════════════════════════════════════╣
║ Min Wait Time:                       0 ms ║
║ Max Wait Time:                     200 ms ║
╠════════════════════════════════════════════╣
║ Starved Tasks:                       0 ║
╚════════════════════════════════════════════╝
```

### 7. **lib.rs** - Library Root

**Purpose**: Module organization and public API

**Module Declarations**:
```rust
pub mod task;
pub mod scheduler;
pub mod executor;
pub mod metrics;
pub mod fifo;
pub mod priority;
```

**Public Exports** (convenience re-exports):
```rust
pub use task::{Task, TaskId, Priority};
pub use scheduler::Scheduler;
pub use executor::Executor;
pub use metrics::{MetricsCollector, TaskMetrics, AggregateMetrics};
pub use fifo::FifoScheduler;
pub use priority::PriorityScheduler;
```

**Benefits of Re-exports**:
- Users don't need to know module structure
- `use articos_scheduler::*` imports main types
- Can reorganize modules without breaking API

---

## Data Flow

### Submission and Execution Flow

```
┌──────────────────────────────────────────────────────────────┐
│ User Code (Control Plane)                                    │
│                                                              │
│  executor.submit_task(task)                                 │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ Executor::submit_task()                                      │
│                                                              │
│  scheduler.submit(task)                                     │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ Scheduler (FIFO or Priority)                                 │
│                                                              │
│  queue.push_back(task)  // FIFO                             │
│  heap.push(task)        // Priority                         │
└──────────────────────────────────────────────────────────────┘
                     
         [Task queued, waiting]
                     
         [scheduler_cycle loop]
                     │
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ Scheduler::next()                                            │
│                                                              │
│  return queue.pop_front()  // FIFO                          │
│  return heap.pop()         // Priority                      │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ ExecutionSlot::assign_task(task)                             │
│                                                              │
│  current_task = Some(task)                                  │
│  started_at = now()                                         │
│  complete_at = now() + task.duration_ms                     │
└──────────────────────────────────────────────────────────────┘
                     
         [Task executing in slot]
                     
                     [time passes]
                     
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ ExecutionSlot::try_complete()                                │
│                                                              │
│  if now() >= complete_at:                                   │
│      return Some((task, started_at))                        │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     ↓
┌──────────────────────────────────────────────────────────────┐
│ MetricsCollector::record_task()                              │
│                                                              │
│  wait = started_at - task.submitted_at                      │
│  exec = completed_at - started_at                           │
│  metrics.push(TaskMetric { wait, exec, ... })               │
└──────────────────────────────────────────────────────────────┘
```

### Time-Based Simulation

The executor uses wall-clock time (CPU clock) to simulate GPU execution:

```
SystemTime::now() → UNIX_EPOCH
        ↓
    u128 nanoseconds
        ↓
    Task submitted_at recorded
        ↓
    Task assigned → started_at recorded
        ↓
    Duration added → complete_at = started_at + duration_ns
        ↓
    Loop: check if now() >= complete_at
        ↓
    When true → task_completed_at recorded
        ↓
    Calculate metrics (differences in timestamps)
```

---

## Key Abstractions

### 1. Task as First-Class Value

**Pattern**: Tasks are immutable once submitted (moved into scheduler)

```rust
pub fn submit(&mut self, task: Task) {
    // Task ownership transferred, can't be modified
    self.queue.push_back(task);
}
```

**Benefits**:
- No task-state corruption
- Clear ownership semantics
- Memory safety

### 2. Scheduler as Strategy Pattern

```rust
pub trait Scheduler {
    fn submit(&mut self, task: Task);
    fn next(&mut self) -> Option<Task>;
    // ... other methods
}

// Runtime selection
let scheduler: Box<dyn Scheduler> = match policy {
    Policy::Fifo => Box::new(FifoScheduler::new()),
    Policy::Priority => Box::new(PriorityScheduler::new()),
};
```

**Benefits**:
- Algorithms encapsulated
- Easy to add new schedulers
- Testable in isolation

### 3. Executor as Orchestrator

The Executor coordinates three responsibilities:

```
Scheduler (where/when to run)
    ↑↓
Executor (orchestrator)
    ↑↓
ExecutionSlots (how to run)
    ↑↓
Metrics (measurements)
```

**Executor's Role**:
- Owns scheduling loop
- Manages slot lifecycle
- Records metrics
- Provides public API

### 4. Metrics as Side Effect

```rust
// Executor records metrics without user involvement
fn schedule_cycle():
    for slot in slots:
        if let Some((task, started)) = slot.try_complete():
            metrics.record_task(&task, started, completed);
```

**Benefits**:
- Transparent collection
- No user overhead
- Complete data capture

---

## Usage Examples

### Example 1: Simple FIFO Simulation

```rust
use articos_scheduler::*;

fn main() {
    // Create scheduler and executor
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit tasks
    for i in 0..10 {
        let task = Task::new(i, 0, 1, 100);
        executor.submit_task(task);
    }
    
    // Run until complete
    executor.run_until_complete(10000);
    
    // Report results
    executor.metrics().print_report();
}
```

### Example 2: Priority Scheduler with Analysis

```rust
use articos_scheduler::*;

fn main() {
    let scheduler = Box::new(PriorityScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit tasks with different priorities
    executor.submit_task(Task::new(1, 255, 1, 100));  // High
    executor.submit_task(Task::new(2, 128, 1, 100));  // Medium
    executor.submit_task(Task::new(3, 1, 1, 100));    // Low
    
    executor.run_until_complete(10000);
    
    // Analyze by priority
    let by_priority = executor.metrics().tasks_by_priority();
    for (priority, tasks) in by_priority {
        let avg_wait: f64 = tasks.iter()
            .map(|t| t.wait_time_ms as f64)
            .sum::<f64>() / tasks.len() as f64;
        println!("Priority {}: {:.2}ms avg wait", priority, avg_wait);
    }
}
```

### Example 3: Custom Simulation with Duration Control

```rust
use articos_scheduler::*;

fn main() {
    let tasks = vec![
        Task::new(1, 5, 1, 50),    // Short
        Task::new(2, 5, 1, 200),   // Long
        Task::new(3, 5, 1, 75),    // Medium
    ];
    
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(2, scheduler);
    
    for task in tasks {
        executor.submit_task(task);
    }
    
    // Run for fixed duration
    executor.run_for_duration(500);
    
    println!("Completed: {}", executor.completed_count());
    println!("Pending: {}", executor.pending_count());
}
```

### Example 4: Comparison Study

```rust
use articos_scheduler::*;

fn compare_schedulers() {
    // Test data
    let create_tasks = |count| {
        (0..count)
            .map(|i| Task::new(i, (i % 10) as u8, 1, 50 + (i % 50) as u64))
            .collect::<Vec<_>>()
    };
    
    // Test FIFO
    println!("=== FIFO Scheduler ===");
    {
        let scheduler = Box::new(FifoScheduler::new());
        let mut executor = Executor::new(4, scheduler);
        
        for task in create_tasks(100) {
            executor.submit_task(task);
        }
        
        executor.run_until_complete(100000);
        executor.metrics().print_report();
    }
    
    // Test Priority
    println!("\n=== Priority Scheduler ===");
    {
        let scheduler = Box::new(PriorityScheduler::new());
        let mut executor = Executor::new(4, scheduler);
        
        for task in create_tasks(100) {
            executor.submit_task(task);
        }
        
        executor.run_until_complete(100000);
        executor.metrics().print_report();
    }
}
```

---

## Integration Points

### 1. Using the Library in Your Code

```rust
// Cargo.toml
[dependencies]
articos-scheduler = { path = "../scheduler" }

// main.rs
use articos_scheduler::*;

fn main() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(8, scheduler);
    // ...
}
```

### 2. Implementing Custom Schedulers

```rust
use articos_scheduler::{Scheduler, Task};

struct MyScheduler {
    // Your scheduling data structure
}

impl Scheduler for MyScheduler {
    fn submit(&mut self, task: Task) {
        // Your logic: priority aging, weighted round-robin, etc.
    }
    
    fn next(&mut self) -> Option<Task> {
        // Your logic: return next task
    }
    
    fn has_pending(&self) -> bool {
        // Your logic
    }
    
    fn pending_count(&self) -> usize {
        // Your logic
    }
    
    fn name(&self) -> &str {
        "MyScheduler"
    }
}

// Use it
let scheduler = Box::new(MyScheduler::new());
let executor = Executor::new(4, scheduler);
```

### 3. Extending Metrics (Future)

To add new metrics, extend `MetricsCollector`:

```rust
pub struct EnhancedMetrics {
    base: MetricsCollector,
    throughput_per_second: f64,        // New: tasks/sec
    priority_fairness_index: f64,      // New: Gini coefficient
    sm_utilization: f64,               // New: % time busy
}
```

### 4. Integrating with Real GPU Code

Future Phase 3 would:
1. Wrap CUDA/HIP runtime calls
2. Measure actual kernel execution times
3. Compare with simulation predictions
4. Validate scheduling decisions

---

## Testing Strategy

### Test Organization

```
scheduler/
├── src/
│   ├── task.rs          [2 tests]
│   ├── scheduler.rs     [1 conceptual test]
│   ├── fifo.rs          [4 tests]
│   ├── priority.rs      [5 tests]
│   ├── metrics.rs       [2 tests]
│   └── executor.rs      [2 tests]
└── tests/
    └── integration_tests.rs [9 tests]
```

### Unit Test Examples

```rust
#[test]
fn test_fifo_order() {
    let mut scheduler = FifoScheduler::new();
    
    // Setup
    for i in 0..5 {
        scheduler.submit(Task::new(i, 0, 1, 100));
    }
    
    // Execute
    let first = scheduler.next();
    
    // Verify
    assert_eq!(first.unwrap().id, 0);
}
```

### Integration Test Examples

```rust
#[test]
fn test_fifo_full_execution() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(2, scheduler);
    
    for i in 0..5 {
        executor.submit_task(Task::new(i, 0, 1, 10));
    }
    
    executor.run_until_complete(10000);
    
    assert_eq!(executor.completed_count(), 5);
    assert_eq!(executor.pending_count(), 0);
}
```

### Test Coverage

- ✅ Task creation and serialization
- ✅ FIFO ordering (order preservation)
- ✅ FIFO fairness (no starvation)
- ✅ Priority ordering (higher first)
- ✅ Priority FIFO within level
- ✅ Execution slot lifecycle
- ✅ Parallel execution
- ✅ Metrics accuracy
- ✅ Starvation detection
- ✅ Empty/edge cases

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_priority_ordering

# With output
cargo test -- --nocapture

# Only integration tests
cargo test --test integration_tests
```

---

## Extension Guide

### Adding a New Scheduler

**Step 1**: Create new file `scheduler/src/your_scheduler.rs`

```rust
use crate::scheduler::Scheduler;
use crate::task::Task;
use std::collections::VecDeque;

pub struct YourScheduler {
    // Your state
}

impl YourScheduler {
    pub fn new() -> Self {
        Self { /* ... */ }
    }
}

impl Default for YourScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for YourScheduler {
    fn submit(&mut self, task: Task) {
        // Your implementation
    }
    
    fn next(&mut self) -> Option<Task> {
        // Your implementation
    }
    
    fn has_pending(&self) -> bool {
        // Your implementation
    }
    
    fn pending_count(&self) -> usize {
        // Your implementation
    }
    
    fn name(&self) -> &str {
        "YourScheduler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_your_scheduler() {
        // Your tests
    }
}
```

**Step 2**: Add to `lib.rs`

```rust
pub mod your_scheduler;
pub use your_scheduler::YourScheduler;
```

**Step 3**: Use it

```rust
let scheduler = Box::new(YourScheduler::new());
let mut executor = Executor::new(4, scheduler);
```

### Scheduler Ideas for Future

1. **Shortest-Job-First (SJF)**
   - Best for minimizing average wait time
   - Requires knowing duration beforehand

2. **Round-Robin**
   - Fair, time-slice based
   - Good for latency fairness

3. **Multi-Level Feedback Queues (MLFQ)**
   - Combines priority with adaptive aging
   - Prevents starvation while favoring short jobs

4. **Weighted Fair Queuing (WFQ)**
   - Proportional resource allocation
   - Good for multi-tenant scenarios

5. **Dynamic Priority Adjustment**
   - Age-based: increase priority over time
   - Load-based: adjust per system state

### Adding Custom Metrics

Extend `MetricsCollector` for new analysis:

```rust
impl MetricsCollector {
    pub fn priority_fairness(&self) -> f64 {
        // Calculate Jain's fairness index
        // Higher = more fair
    }
    
    pub fn throughput(&self) -> f64 {
        // Tasks completed per unit time
    }
    
    pub fn response_time_95th_percentile(&self) -> u128 {
        // For SLA compliance
    }
}
```

### Integrating with External Systems

Future integrations:

1. **JSON Export**
   ```rust
   let json = serde_json::to_string(&executor.metrics().aggregate())?;
   std::fs::write("metrics.json", json)?;
   ```

2. **Real-time Monitoring**
   ```rust
   // Publish metrics to Prometheus, Grafana, etc.
   prometheus_gauge!("scheduler_pending_tasks").set(
       executor.pending_count() as f64
   );
   ```

3. **GPU Integration**
   ```rust
   // Wrap real CUDA kernel execution
   unsafe {
       cudaLaunchKernel(...);  // Real GPU
       metrics.record_task(...); // Measure
   }
   ```

---

## Summary of Key Concepts

### Core Abstractions

| Concept | Rust Type | Purpose |
|---------|-----------|---------|
| Work Unit | `Task` | Schedulable unit with priority |
| Policy | `Scheduler` trait | Determines execution order |
| Resources | `ExecutionSlot` | Where tasks execute |
| Runtime | `Executor` | Coordinates everything |
| Analysis | `MetricsCollector` | Measures performance |

### Data Structures

| Component | Structure | Complexity |
|-----------|-----------|-----------|
| FIFO Queue | `VecDeque` | O(1) |
| Priority Queue | `BinaryHeap` | O(log n) |
| Execution Slots | `Vec` | O(1) lookup |
| Metrics Storage | `Vec` | O(n) analysis |

### Key Behaviors

1. **No Preemption** - Tasks run to completion
2. **Non-blocking Submission** - submit() returns immediately
3. **Lazy Scheduling** - schedule_cycle() triggered by user
4. **Transparent Metrics** - Collected automatically

---

## Glossary

| Term | Definition | GPU Analog |
|------|------------|-----------|
| Task | Unit of work to schedule | Kernel launch |
| Queue | Waiting tasks | Command buffer |
| Priority | Urgency level (0-255) | Stream priority |
| SM | Streaming Multiprocessor | Execution resource |
| Slot | Simulated SM | Execution unit |
| Wait Time | Queue duration | Submission to execution |
| Starvation | Indefinite wait | Low-priority blocking |
| Metric | Measured statistic | Profile data |
| Cycle | One scheduling iteration | Hardware clock cycle |

---

## Performance Characteristics

### Asymptotic Complexity

| Operation | FIFO | Priority | Comment |
|-----------|------|----------|---------|
| Submit | O(1) | O(log n) | Heap vs deque |
| Next | O(1) | O(log n) | Pop operation |
| Check Pending | O(1) | O(1) | Size tracking |
| Get Metrics | O(n) | O(n) | Full scan needed |

### Memory Usage

```
Per Task: ~100 bytes (id, priority, resources, timestamps)
Per Slot: ~64 bytes (state, pointers)
Per Metric: ~80 bytes (timestamps, values)

Small workloads: <1 MB
Medium (1000s tasks): ~10 MB
Large (millions): Could be GB-scale
```

---

## Architecture Decision Record (ADR)

### Why CPU Simulation Instead of Real GPU?

**Pros**:
- ✅ Runs on any machine (no GPU required)
- ✅ Deterministic (easier testing)
- ✅ Fast iteration (no compilation overhead)
- ✅ Easy visualization (CPU stdout)

**Cons**:
- ❌ Different timing characteristics
- ❌ Doesn't test actual GPU behavior
- ❌ Memory model differs

**Decision**: Phase 1 simulation, Phase 3 real GPU integration

### Why Trait-Based Abstraction?

**Pros**:
- ✅ Multiple implementations
- ✅ Easy testing with mocks
- ✅ Follows Rust idioms

**Cons**:
- ❌ Runtime indirection (minor perf cost)

**Decision**: Accepted cost for flexibility

### Why VecDeque for FIFO, BinaryHeap for Priority?

**VecDeque**:
- O(1) push_back and pop_front
- Perfect for FIFO

**BinaryHeap**:
- O(log n) push and pop
- Maintains heap property for priority ordering

**Decision**: Standard library algorithms match perfectly

---

## Conclusion

ARTICOS Phase 1 provides a clean, extensible foundation for GPU scheduler development. The trait-based design enables:

- ✅ Multiple scheduler implementations
- ✅ Easy testing and validation
- ✅ Clear separation of concerns
- ✅ Foundation for real GPU integration

The codebase is production-quality with comprehensive documentation, full test coverage, and real-world simulation scenarios.

---

**For questions or extensions, see**:
- [README.md](README.md) - Project overview
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Deep architecture
- [docs/QUICKSTART.md](docs/QUICKSTART.md) - Getting started
- [docs/GPU_CONCEPTS.md](docs/GPU_CONCEPTS.md) - GPU fundamentals
- [scheduler/src/lib.rs](scheduler/src/lib.rs) - Module documentation
