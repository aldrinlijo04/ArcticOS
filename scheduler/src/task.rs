use serde::{Deserialize, Serialize};

/// Unique identifier for a task
/// In a real GPU runtime, this would correspond to a kernel launch ID
pub type TaskId = u64;

/// Priority level for task scheduling
/// Higher number = higher priority (0 is lowest)
/// GPU analogy: Stream priority in CUDA/HIP
pub type Priority = u8;

/// Resource requirement representing compute units needed
/// GPU analogy: Number of thread blocks or compute units required
pub type ResourceRequirement = u32;

/// Simulated task duration in milliseconds
/// GPU analogy: Estimated kernel execution time
pub type DurationMs = u64;

/// Represents a schedulable workload unit
/// GPU analogy: Similar to a CUDA kernel launch or compute dispatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task
    pub id: TaskId,
    
    /// Scheduling priority (higher = more important)
    /// Range: 0-255, where 255 is highest priority
    pub priority: Priority,
    
    /// Number of compute units (SMs) required
    /// In real GPU: this would be derived from grid dimensions
    pub resource_requirement: ResourceRequirement,
    
    /// Expected execution duration in milliseconds
    /// In simulation: used to model compute time
    pub duration_ms: DurationMs,
    
    /// Timestamp when task was submitted (nanoseconds since epoch)
    pub submitted_at: u128,
}

impl Task {
    /// Create a new task with the given parameters
    pub fn new(
        id: TaskId,
        priority: Priority,
        resource_requirement: ResourceRequirement,
        duration_ms: DurationMs,
    ) -> Self {
        Self {
            id,
            priority,
            resource_requirement,
            duration_ms,
            submitted_at: current_time_nanos(),
        }
    }
    
    /// Create a task with default priority
    pub fn with_duration(id: TaskId, duration_ms: DurationMs) -> Self {
        Self::new(id, 0, 1, duration_ms)
    }
}

/// Get current time in nanoseconds
/// Used for precise timing measurements in simulation
pub fn current_time_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// Convert nanoseconds to milliseconds
pub fn nanos_to_millis(nanos: u128) -> u128 {
    nanos / 1_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(1, 5, 2, 100);
        assert_eq!(task.id, 1);
        assert_eq!(task.priority, 5);
        assert_eq!(task.resource_requirement, 2);
        assert_eq!(task.duration_ms, 100);
        assert!(task.submitted_at > 0);
    }

    #[test]
    fn test_task_with_duration() {
        let task = Task::with_duration(42, 250);
        assert_eq!(task.id, 42);
        assert_eq!(task.priority, 0);
        assert_eq!(task.resource_requirement, 1);
        assert_eq!(task.duration_ms, 250);
    }
}
