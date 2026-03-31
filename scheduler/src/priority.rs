use crate::scheduler::Scheduler;
use crate::task::{Priority, Task};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Wrapper for Task to enable priority-based ordering in BinaryHeap
/// 
/// GPU analogy: Models CUDA stream priorities or compute queue priorities
/// Higher priority values are scheduled before lower priority values
#[derive(Clone)]
struct PriorityTask {
    task: Task,
}

impl PriorityTask {
    fn new(task: Task) -> Self {
        Self { task }
    }
}

// Implement ordering based on priority (higher priority first)
// If priorities are equal, use submission time (earlier first - FIFO within priority)
impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by priority (higher is better)
        match self.task.priority.cmp(&other.task.priority) {
            Ordering::Equal => {
                // If equal priority, older task goes first (FIFO)
                // Reverse comparison because earlier timestamp should be "greater" in max-heap
                match other.task.submitted_at.cmp(&self.task.submitted_at) {
                    Ordering::Equal => {
                        // If timestamps equal (fast submission), use task ID as tiebreaker
                        other.task.id.cmp(&self.task.id)
                    }
                    ord => ord,
                }
            }
            other => other,
        }
    }
}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for PriorityTask {}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority 
            && self.task.submitted_at == other.task.submitted_at
    }
}

/// Priority-based scheduler implementation
/// 
/// GPU analogy: Similar to CUDA stream priorities (cudaStreamCreateWithPriority)
/// or Vulkan queue priorities. High-priority tasks are scheduled before low-priority
/// tasks, enabling critical workloads to execute with lower latency.
/// 
/// Characteristics:
/// - Priority-aware: Higher priority tasks execute first
/// - FIFO within priority: Tasks with same priority use FIFO ordering
/// - Starvation possible: Low-priority tasks may wait indefinitely if high-priority
///   tasks keep arriving
/// - O(log n) operations: Uses binary heap for efficient priority queue
/// 
/// Use case: Workloads with varying importance levels (e.g., real-time rendering
/// at high priority, background compute at low priority)
pub struct PriorityScheduler {
    /// Binary heap for priority-based task ordering
    /// BinaryHeap is max-heap, so highest priority is at top
    heap: BinaryHeap<PriorityTask>,
    
    /// Track number of tasks at each priority level
    /// Used for analysis and starvation detection
    priority_counts: [usize; 256],
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            priority_counts: [0; 256],
        }
    }
    
    /// Create a priority scheduler with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(capacity),
            priority_counts: [0; 256],
        }
    }
    
    /// Get distribution of tasks by priority level
    pub fn priority_distribution(&self) -> Vec<(Priority, usize)> {
        self.priority_counts
            .iter()
            .enumerate()
            .filter(|(_, &count)| count > 0)
            .map(|(priority, &count)| (priority as Priority, count))
            .collect()
    }
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler for PriorityScheduler {
    fn submit(&mut self, task: Task) {
        let priority_idx = task.priority as usize;
        self.priority_counts[priority_idx] += 1;
        
        tracing::trace!(
            task_id = task.id,
            priority = task.priority,
            duration_ms = task.duration_ms,
            queue_len = self.heap.len(),
            "Task submitted to Priority scheduler"
        );
        
        self.heap.push(PriorityTask::new(task));
    }
    
    fn next(&mut self) -> Option<Task> {
        let priority_task = self.heap.pop();
        
        if let Some(ref pt) = priority_task {
            let priority_idx = pt.task.priority as usize;
            self.priority_counts[priority_idx] = 
                self.priority_counts[priority_idx].saturating_sub(1);
            
            tracing::trace!(
                task_id = pt.task.id,
                priority = pt.task.priority,
                remaining = self.heap.len(),
                "Task dequeued from Priority scheduler"
            );
        }
        
        priority_task.map(|pt| pt.task)
    }
    
    fn has_pending(&self) -> bool {
        !self.heap.is_empty()
    }
    
    fn pending_count(&self) -> usize {
        self.heap.len()
    }
    
    fn name(&self) -> &str {
        "Priority"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        let mut scheduler = PriorityScheduler::new();
        
        // Submit tasks with different priorities
        scheduler.submit(Task::new(1, 5, 1, 100));   // Medium
        scheduler.submit(Task::new(2, 10, 1, 100));  // High
        scheduler.submit(Task::new(3, 1, 1, 100));   // Low
        scheduler.submit(Task::new(4, 10, 1, 100));  // High
        scheduler.submit(Task::new(5, 3, 1, 100));   // Low-Medium
        
        assert_eq!(scheduler.pending_count(), 5);
        
        // Should come out in priority order (high to low)
        // Within same priority, FIFO order
        let task2 = scheduler.next().unwrap();
        assert_eq!(task2.id, 2); // Priority 10, first
        
        let task4 = scheduler.next().unwrap();
        assert_eq!(task4.id, 4); // Priority 10, second
        
        let task1 = scheduler.next().unwrap();
        assert_eq!(task1.id, 1); // Priority 5
        
        let task5 = scheduler.next().unwrap();
        assert_eq!(task5.id, 5); // Priority 3
        
        let task3 = scheduler.next().unwrap();
        assert_eq!(task3.id, 3); // Priority 1
        
        assert!(!scheduler.has_pending());
    }

    #[test]
    fn test_priority_fifo_within_level() {
        let mut scheduler = PriorityScheduler::new();
        
        // Submit multiple tasks with same priority
        for i in 0..5 {
            scheduler.submit(Task::new(i, 5, 1, 100));
        }
        
        // Should maintain FIFO order within same priority
        for i in 0..5 {
            let task = scheduler.next().unwrap();
            assert_eq!(task.id, i);
        }
    }

    #[test]
    fn test_priority_distribution() {
        let mut scheduler = PriorityScheduler::new();
        
        scheduler.submit(Task::new(1, 5, 1, 100));
        scheduler.submit(Task::new(2, 5, 1, 100));
        scheduler.submit(Task::new(3, 10, 1, 100));
        
        let dist = scheduler.priority_distribution();
        assert_eq!(dist.len(), 2);
        
        // Check counts
        assert!(dist.contains(&(5, 2)));
        assert!(dist.contains(&(10, 1)));
    }

    #[test]
    fn test_priority_empty_queue() {
        let mut scheduler = PriorityScheduler::new();
        assert!(!scheduler.has_pending());
        assert_eq!(scheduler.pending_count(), 0);
        assert!(scheduler.next().is_none());
    }

    #[test]
    fn test_max_priority() {
        let mut scheduler = PriorityScheduler::new();
        
        scheduler.submit(Task::new(1, 0, 1, 100));     // Min priority
        scheduler.submit(Task::new(2, 255, 1, 100));   // Max priority
        scheduler.submit(Task::new(3, 128, 1, 100));   // Mid priority
        
        assert_eq!(scheduler.next().unwrap().id, 2); // Max first
        assert_eq!(scheduler.next().unwrap().id, 3); // Mid second
        assert_eq!(scheduler.next().unwrap().id, 1); // Min last
    }
}
