//! # ARTICOS Scheduler
//! 
//! GPU-first runtime scheduler simulation for ARTICOS Phase 1.
//! 
//! This library provides a CPU-based simulation of GPU runtime scheduling,
//! modeling the behavior of GPU hardware schedulers (e.g., NVIDIA SM scheduler,
//! AMD GCN scheduler) without requiring actual GPU hardware.
//! 
//! ## Architecture
//! 
//! - **Task**: Represents a schedulable workload unit (analogous to kernel launch)
//! - **Scheduler**: Trait for implementing scheduling policies (FIFO, Priority, etc.)
//! - **Executor**: Runtime that manages execution slots (simulated SMs)
//! - **Metrics**: Performance measurement and starvation detection
//! 
//! ## Schedulers
//! 
//! - **FIFO**: First-In-First-Out, fair but non-optimal
//! - **Priority**: Priority-based with starvation potential
//! 
//! ## Example
//! 
//! ```no_run
//! use articos_scheduler::*;
//! 
//! // Create scheduler and executor
//! let scheduler = Box::new(fifo::FifoScheduler::new());
//! let mut executor = executor::Executor::new(4, scheduler);
//! 
//! // Submit tasks
//! executor.submit_task(task::Task::new(1, 0, 1, 100));
//! executor.submit_task(task::Task::new(2, 0, 1, 200));
//! 
//! // Run simulation
//! executor.run_until_complete(10000);
//! 
//! // Analyze metrics
//! executor.metrics().print_report();
//! ```

pub mod task;
pub mod scheduler;
pub mod executor;
pub mod metrics;
pub mod fifo;
pub mod priority;

// Re-export common types for convenience
pub use task::{Task, TaskId, Priority};
pub use scheduler::Scheduler;
pub use executor::Executor;
pub use metrics::{MetricsCollector, TaskMetrics, AggregateMetrics};
pub use fifo::FifoScheduler;
pub use priority::PriorityScheduler;
