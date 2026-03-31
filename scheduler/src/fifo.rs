use crate::scheduler::Scheduler;
use crate::task::Task;
use std::collections::VecDeque;

/// First-In-First-Out (FIFO) scheduler implementation
/// 
/// GPU analogy: Simplest scheduling policy, similar to default CUDA stream behavior
/// Tasks are executed in the exact order they are submitted, regardless of priority
/// or resource requirements.
/// 
/// Characteristics:
/// - Fair: Every task eventually executes (no starvation)
/// - Simple: O(1) enqueue and dequeue operations
/// - Predictable: Execution order matches submission order
/// - Non-optimal: Doesn't consider priority or optimize for throughput
/// 
/// Use case: Suitable for workloads where all tasks have equal importance
/// and arrival order represents natural execution dependencies
pub struct FifoScheduler {
    /// Queue of tasks waiting to be executed
    /// Front of queue = next task to execute
    queue: VecDeque<Task>,
}

impl FifoScheduler {
    /// Create a new FIFO scheduler
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    
    /// Create a FIFO scheduler with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
        }
    }
}

impl Default for FifoScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for FifoScheduler {
    fn submit(&mut self, task: Task) {
        tracing::trace!(
            task_id = task.id,
            priority = task.priority,
            duration_ms = task.duration_ms,
            queue_len = self.queue.len(),
            "Task submitted to FIFO scheduler"
        );
        
        self.queue.push_back(task);
    }
    
    fn next(&mut self) -> Option<Task> {
        let task = self.queue.pop_front();
        
        if let Some(ref t) = task {
            tracing::trace!(
                task_id = t.id,
                remaining = self.queue.len(),
                "Task dequeued from FIFO scheduler"
            );
        }
        
        task
    }
    
    fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }
    
    fn pending_count(&self) -> usize {
        self.queue.len()
    }
    
    fn name(&self) -> &str {
        "FIFO"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_order() {
        let mut scheduler = FifoScheduler::new();
        
        // Submit tasks in order
        for i in 0..5 {
            scheduler.submit(Task::new(i, 0, 1, 100));
        }
        
        assert_eq!(scheduler.pending_count(), 5);
        
        // Tasks should come out in same order
        for i in 0..5 {
            let task = scheduler.next().unwrap();
            assert_eq!(task.id, i);
        }
        
        assert_eq!(scheduler.pending_count(), 0);
        assert!(scheduler.next().is_none());
    }

    #[test]
    fn test_fifo_ignores_priority() {
        let mut scheduler = FifoScheduler::new();
        
        // Submit tasks with different priorities
        scheduler.submit(Task::new(1, 0, 1, 100));  // Low priority
        scheduler.submit(Task::new(2, 255, 1, 100)); // High priority
        scheduler.submit(Task::new(3, 10, 1, 100));  // Medium priority
        
        // Should still come out in submission order
        assert_eq!(scheduler.next().unwrap().id, 1);
        assert_eq!(scheduler.next().unwrap().id, 2);
        assert_eq!(scheduler.next().unwrap().id, 3);
    }

    #[test]
    fn test_fifo_empty_queue() {
        let mut scheduler = FifoScheduler::new();
        assert!(!scheduler.has_pending());
        assert_eq!(scheduler.pending_count(), 0);
        assert!(scheduler.next().is_none());
    }

    #[test]
    fn test_fifo_with_capacity() {
        let scheduler = FifoScheduler::with_capacity(100);
        assert_eq!(scheduler.pending_count(), 0);
    }
}
