use crate::metrics::MetricsCollector;
use crate::scheduler::Scheduler;
use crate::task::{current_time_nanos, Task};
use crate::gpu::executor::GpuExecutor;

use std::time::Duration;

pub struct Executor {
    scheduler: Box<dyn Scheduler>,
    metrics: MetricsCollector,
    completed_tasks: usize,
    gpu: GpuExecutor,
    num_slots: usize,
}

impl Executor {
    pub fn new(num_slots: usize, scheduler: Box<dyn Scheduler>) -> Self {
        assert!(num_slots > 0, "Must have at least one execution slot");

        Self {
            scheduler,
            metrics: MetricsCollector::new(),
            completed_tasks: 0,

            // ✅ FIX 1: pass num_slots
            gpu: GpuExecutor::new(num_slots).expect("Failed to initialize GPU"),

            num_slots,
        }
    }

    pub fn submit_task(&mut self, task: Task) {
        self.scheduler.submit(task);
    }

    fn schedule_cycle(&mut self) -> usize {
        let mut executed = 0;

        for _ in 0..self.num_slots {
            if let Some(task) = self.scheduler.next() {
                tracing::debug!(
                    task_id = task.id,
                    priority = task.priority,
                    duration_ms = task.duration_ms,
                    "Task executing on GPU"
                );

                let start = current_time_nanos();

                self.gpu.execute(&task).expect("GPU execution failed");

                let end = current_time_nanos();

                self.metrics.record_task(&task, start, end);
                self.completed_tasks += 1;
                executed += 1;
            }
        }

        executed
    }

    pub fn run_until_complete(&mut self, max_iterations: usize) {
        let scheduler_name = self.scheduler.name();

        tracing::info!(
            scheduler = scheduler_name,
            num_slots = self.num_slots,
            "Starting executor"
        );

        let mut iterations = 0;

        while iterations < max_iterations {
            iterations += 1;

            let executed = self.schedule_cycle();
            let no_pending = !self.scheduler.has_pending();

            if executed == 0 && no_pending {
                break;
            }

            std::thread::sleep(Duration::from_millis(1));
        }

        // ✅ FIX 2: wait for GPU to finish
        self.gpu.synchronize_all();

        tracing::info!(
            iterations,
            completed_tasks = self.completed_tasks,
            "Execution complete"
        );

        if iterations >= max_iterations {
            tracing::warn!("Execution terminated: max iterations reached");
        }
    }

    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    pub fn completed_count(&self) -> usize {
        self.completed_tasks
    }

    pub fn pending_count(&self) -> usize {
        self.scheduler.pending_count()
    }

    pub fn total_slots(&self) -> usize {
        self.num_slots
    }
}