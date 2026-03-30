use crate::metrics::MetricsCollector;
use crate::scheduler::Scheduler;
use crate::task::{current_time_nanos, Task};
use std::thread;
use std::time::Duration;

/// Represents a single execution slot (Streaming Multiprocessor in GPU terms)
/// 
/// GPU analogy: Models an NVIDIA SM or AMD Compute Unit (CU)
/// In real hardware, each SM can execute multiple warps/wavefronts,
/// but for Phase 1 simulation, we model one task per slot for simplicity
#[derive(Debug)]
struct ExecutionSlot {
    /// Slot identifier (0..num_slots)
    id: usize,
    
    /// Currently executing task, if any
    current_task: Option<Task>,
    
    /// When the current task started executing
    started_at: Option<u128>,
    
    /// When the current task should complete
    complete_at: Option<u128>,
}

impl ExecutionSlot {
    fn new(id: usize) -> Self {
        Self {
            id,
            current_task: None,
            started_at: None,
            complete_at: None,
        }
    }
    
    /// Check if slot is available for new work
    fn is_available(&self) -> bool {
        self.current_task.is_none()
    }
    
    /// Assign a task to this slot
    /// Returns the start timestamp
    fn assign_task(&mut self, task: Task) -> u128 {
        let now = current_time_nanos();
        let complete_at = now + (task.duration_ms as u128 * 1_000_000); // Convert ms to ns
        
        self.current_task = Some(task);
        self.started_at = Some(now);
        self.complete_at = Some(complete_at);
        
        now
    }
    
    /// Check if current task has completed
    /// Returns Some((task, started_at)) if completed, None otherwise
    fn try_complete(&mut self) -> Option<(Task, u128)> {
        if let Some(complete_at) = self.complete_at {
            let now = current_time_nanos();
            if now >= complete_at {
                let task = self.current_task.take().unwrap();
                let started_at = self.started_at.take().unwrap();
                self.complete_at = None;
                return Some((task, started_at));
            }
        }
        None
    }
}

/// Runtime executor that manages simulated SM execution slots
/// 
/// GPU analogy: Models the GPU's hardware scheduler and execution resources
/// - Fixed number of SMs (execution slots)
/// - No preemption once a task starts executing
/// - Tasks run to completion
/// - Scheduler provides next task to run
pub struct Executor {
    /// Simulated execution slots (SMs/CUs)
    slots: Vec<ExecutionSlot>,
    
    /// The scheduler implementation to use
    scheduler: Box<dyn Scheduler>,
    
    /// Metrics collector for performance analysis
    metrics: MetricsCollector,
    
    /// Total number of tasks completed
    completed_tasks: usize,
}

impl Executor {
    /// Create a new executor with specified number of SM slots
    /// 
    /// # Arguments
    /// * `num_slots` - Number of simulated SMs (typically 4-108 in real GPUs)
    /// * `scheduler` - Scheduler implementation to use for task ordering
    pub fn new(num_slots: usize, scheduler: Box<dyn Scheduler>) -> Self {
        assert!(num_slots > 0, "Must have at least one execution slot");
        
        let slots = (0..num_slots).map(ExecutionSlot::new).collect();
        
        Self {
            slots,
            scheduler,
            metrics: MetricsCollector::new(),
            completed_tasks: 0,
        }
    }
    
    /// Submit a task to the scheduler
    pub fn submit_task(&mut self, task: Task) {
        self.scheduler.submit(task);
    }
    
    /// Run one scheduling cycle
    /// - Check for completed tasks
    /// - Try to schedule new tasks on available slots
    /// 
    /// Returns number of tasks scheduled in this cycle
    fn schedule_cycle(&mut self) -> usize {
        let mut scheduled = 0;
        
        // First, check for completed tasks and free up slots
        for slot in &mut self.slots {
            if let Some((task, started_at)) = slot.try_complete() {
                let completed_at = current_time_nanos();
                self.metrics.record_task(&task, started_at, completed_at);
                self.completed_tasks += 1;
                
                tracing::debug!(
                    task_id = task.id,
                    slot_id = slot.id,
                    "Task completed"
                );
            }
        }
        
        // Then, try to schedule new tasks on available slots
        for slot in &mut self.slots {
            if slot.is_available() {
                if let Some(task) = self.scheduler.next() {
                    tracing::debug!(
                        task_id = task.id,
                        slot_id = slot.id,
                        priority = task.priority,
                        duration_ms = task.duration_ms,
                        "Task scheduled"
                    );
                    
                    slot.assign_task(task);
                    scheduled += 1;
                }
            }
        }
        
        scheduled
    }
    
    /// Run the simulation until all tasks complete
    /// 
    /// # Arguments
    /// * `max_iterations` - Maximum scheduling cycles to run (prevents infinite loops)
    pub fn run_until_complete(&mut self, max_iterations: usize) {
        let scheduler_name = self.scheduler.name();
        tracing::info!(
            scheduler = scheduler_name,
            num_slots = self.slots.len(),
            "Starting executor"
        );
        
        let mut iterations = 0;
        
        while iterations < max_iterations {
            iterations += 1;
            
            // Run a scheduling cycle
            let _scheduled = self.schedule_cycle();
            
            // Check if we're done
            let all_slots_idle = self.slots.iter().all(|s| s.is_available());
            let no_pending = !self.scheduler.has_pending();
            
            if all_slots_idle && no_pending {
                tracing::info!(
                    iterations,
                    completed_tasks = self.completed_tasks,
                    "Simulation complete"
                );
                break;
            }
            
            // Small sleep to simulate passage of time
            // In real GPU, this would be hardware clock cycles
            thread::sleep(Duration::from_micros(100));
        }
        
        if iterations >= max_iterations {
            tracing::warn!("Simulation terminated: max iterations reached");
        }
    }
    
    /// Run simulation for a fixed duration
    /// 
    /// # Arguments
    /// * `duration_ms` - How long to run the simulation
    pub fn run_for_duration(&mut self, duration_ms: u64) {
        let start = current_time_nanos();
        let end = start + (duration_ms as u128 * 1_000_000);
        
        while current_time_nanos() < end {
            self.schedule_cycle();
            thread::sleep(Duration::from_micros(100));
        }
    }
    
    /// Get reference to metrics collector
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }
    
    /// Get number of available execution slots
    pub fn available_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.is_available()).count()
    }
    
    /// Get total number of execution slots
    pub fn total_slots(&self) -> usize {
        self.slots.len()
    }
    
    /// Get number of tasks completed so far
    pub fn completed_count(&self) -> usize {
        self.completed_tasks
    }
    
    /// Get number of tasks currently executing
    pub fn executing_count(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_available()).count()
    }
    
    /// Get number of tasks pending in scheduler
    pub fn pending_count(&self) -> usize {
        self.scheduler.pending_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fifo::FifoScheduler;

    #[test]
    fn test_execution_slot_lifecycle() {
        let mut slot = ExecutionSlot::new(0);
        assert!(slot.is_available());
        
        let task = Task::new(1, 0, 1, 10);
        slot.assign_task(task);
        assert!(!slot.is_available());
        
        // Shouldn't complete immediately
        assert!(slot.try_complete().is_none());
    }

    #[test]
    fn test_executor_creation() {
        let scheduler = Box::new(FifoScheduler::new());
        let executor = Executor::new(4, scheduler);
        assert_eq!(executor.total_slots(), 4);
        assert_eq!(executor.available_slots(), 4);
        assert_eq!(executor.completed_count(), 0);
    }
}


