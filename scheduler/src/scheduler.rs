use crate::task::Task;

/// Core scheduling interface for ARTICOS
/// 
/// GPU analogy: Similar to CUDA Stream or HIP Queue management
/// Each scheduler implementation decides how to order and dispatch tasks
/// to available compute resources (simulated SMs in Phase 1)
pub trait Scheduler: Send + Sync {
    /// Submit a new task to the scheduler
    /// 
    /// GPU analogy: Equivalent to cudaLaunchKernel or kernel<<<>>> dispatch
    /// Task is added to scheduler's queue for future execution
    fn submit(&mut self, task: Task);
    
    /// Get the next task to execute, if available
    /// 
    /// Returns None if no tasks are ready or resources are unavailable
    /// GPU analogy: Hardware scheduler picking next wavefront/warp to issue
    fn next(&mut self) -> Option<Task>;
    
    /// Check if scheduler has any pending tasks
    fn has_pending(&self) -> bool;
    
    /// Get number of pending tasks in queue
    fn pending_count(&self) -> usize;
    
    /// Get scheduler name for identification
    fn name(&self) -> &str;
}

/// Result of attempting to schedule a task on executor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleResult {
    /// Task was successfully scheduled on an execution slot
    Scheduled,
    
    /// No available execution slots (all SMs busy)
    NoSlots,
    
    /// No tasks available in scheduler queue
    NoTasks,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper to verify Scheduler trait is object-safe
    fn _assert_object_safe(_: &dyn Scheduler) {}
}
