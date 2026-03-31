use crate::task::{current_time_nanos, nanos_to_millis, Task, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metrics collected for each task during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Task identifier
    pub task_id: TaskId,
    
    /// Time spent waiting in queue (milliseconds)
    pub wait_time_ms: u128,
    
    /// Actual execution time (milliseconds)
    pub execution_time_ms: u128,
    
    /// Total time from submission to completion (milliseconds)
    pub total_time_ms: u128,
    
    /// Timestamp when task was submitted
    pub submitted_at: u128,
    
    /// Timestamp when task started execution
    pub started_at: u128,
    
    /// Timestamp when task completed
    pub completed_at: u128,
    
    /// Task priority
    pub priority: u8,
}

impl TaskMetrics {
    /// Create metrics from task execution timestamps
    pub fn new(task: &Task, started_at: u128, completed_at: u128) -> Self {
        let wait_time_ns = started_at.saturating_sub(task.submitted_at);
        let execution_time_ns = completed_at.saturating_sub(started_at);
        let total_time_ns = completed_at.saturating_sub(task.submitted_at);
        
        Self {
            task_id: task.id,
            wait_time_ms: nanos_to_millis(wait_time_ns),
            execution_time_ms: nanos_to_millis(execution_time_ns),
            total_time_ms: nanos_to_millis(total_time_ns),
            submitted_at: task.submitted_at,
            started_at,
            completed_at,
            priority: task.priority,
        }
    }
}

/// Aggregate metrics across all tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    /// Total number of tasks processed
    pub total_tasks: usize,
    
    /// Average wait time across all tasks (milliseconds)
    pub avg_wait_time_ms: f64,
    
    /// Average execution time across all tasks (milliseconds)
    pub avg_execution_time_ms: f64,
    
    /// Average total time across all tasks (milliseconds)
    pub avg_total_time_ms: f64,
    
    /// Maximum wait time observed (milliseconds)
    pub max_wait_time_ms: u128,
    
    /// Minimum wait time observed (milliseconds)
    pub min_wait_time_ms: u128,
    
    /// Number of tasks that experienced starvation
    /// Starvation: wait time > 10x average execution time
    pub starved_tasks: usize,
    
    /// Total simulation time (milliseconds)
    pub total_simulation_time_ms: u128,
}

/// Collects and analyzes metrics for scheduler performance evaluation
/// GPU analogy: Similar to NVIDIA Nsight profiler or rocProf
pub struct MetricsCollector {
    task_metrics: Vec<TaskMetrics>,
    simulation_start: u128,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            task_metrics: Vec::new(),
            simulation_start: current_time_nanos(),
        }
    }
    
    /// Record metrics for a completed task
    pub fn record_task(&mut self, task: &Task, started_at: u128, completed_at: u128) {
        let metrics = TaskMetrics::new(task, started_at, completed_at);
        self.task_metrics.push(metrics);
    }
    
    /// Get all task metrics
    pub fn task_metrics(&self) -> &[TaskMetrics] {
        &self.task_metrics
    }
    
    /// Calculate aggregate metrics across all tasks
    pub fn aggregate(&self) -> AggregateMetrics {
        if self.task_metrics.is_empty() {
            return AggregateMetrics {
                total_tasks: 0,
                avg_wait_time_ms: 0.0,
                avg_execution_time_ms: 0.0,
                avg_total_time_ms: 0.0,
                max_wait_time_ms: 0,
                min_wait_time_ms: 0,
                starved_tasks: 0,
                total_simulation_time_ms: 0,
            };
        }
        
        let total_wait: u128 = self.task_metrics.iter().map(|m| m.wait_time_ms).sum();
        let total_exec: u128 = self.task_metrics.iter().map(|m| m.execution_time_ms).sum();
        let total_time: u128 = self.task_metrics.iter().map(|m| m.total_time_ms).sum();
        
        let count = self.task_metrics.len() as f64;
        let avg_exec = total_exec as f64 / count;
        
        let max_wait = self.task_metrics.iter().map(|m| m.wait_time_ms).max().unwrap_or(0);
        let min_wait = self.task_metrics.iter().map(|m| m.wait_time_ms).min().unwrap_or(0);
        
        // Detect starvation: tasks that waited more than 10x average execution time
        let starvation_threshold = (avg_exec * 10.0) as u128;
        let starved = self.task_metrics.iter()
            .filter(|m| m.wait_time_ms > starvation_threshold)
            .count();
        
        let simulation_time = nanos_to_millis(
            current_time_nanos().saturating_sub(self.simulation_start)
        );
        
        AggregateMetrics {
            total_tasks: self.task_metrics.len(),
            avg_wait_time_ms: total_wait as f64 / count,
            avg_execution_time_ms: avg_exec,
            avg_total_time_ms: total_time as f64 / count,
            max_wait_time_ms: max_wait,
            min_wait_time_ms: min_wait,
            starved_tasks: starved,
            total_simulation_time_ms: simulation_time,
        }
    }
    
    /// Print detailed metrics report
    pub fn print_report(&self) {
        let agg = self.aggregate();
        
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║           ARTICOS Scheduler Metrics Report                ║");
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Total Tasks Processed: {:>35} ║", agg.total_tasks);
        println!("║ Simulation Time: {:>38} ms ║", agg.total_simulation_time_ms);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Average Wait Time: {:>35.2} ms ║", agg.avg_wait_time_ms);
        println!("║ Average Execution Time: {:>31.2} ms ║", agg.avg_execution_time_ms);
        println!("║ Average Total Time: {:>34.2} ms ║", agg.avg_total_time_ms);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Min Wait Time: {:>39} ms ║", agg.min_wait_time_ms);
        println!("║ Max Wait Time: {:>39} ms ║", agg.max_wait_time_ms);
        println!("╠════════════════════════════════════════════════════════════╣");
        println!("║ Starved Tasks: {:>39} ║", agg.starved_tasks);
        if agg.starved_tasks > 0 {
            let starvation_rate = (agg.starved_tasks as f64 / agg.total_tasks as f64) * 100.0;
            println!("║ Starvation Rate: {:>35.2} % ║", starvation_rate);
        }
        println!("╚════════════════════════════════════════════════════════════╝\n");
    }
    
    /// Get tasks grouped by priority level
    pub fn tasks_by_priority(&self) -> HashMap<u8, Vec<&TaskMetrics>> {
        let mut by_priority: HashMap<u8, Vec<&TaskMetrics>> = HashMap::new();
        for metrics in &self.task_metrics {
            by_priority.entry(metrics.priority).or_insert_with(Vec::new).push(metrics);
        }
        by_priority
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_empty() {
        let collector = MetricsCollector::new();
        let agg = collector.aggregate();
        assert_eq!(agg.total_tasks, 0);
        assert_eq!(agg.avg_wait_time_ms, 0.0);
    }

    #[test]
    fn test_task_metrics_calculation() {
        let task = Task::new(1, 5, 1, 100);
        let started_at = task.submitted_at + 50_000_000; // 50ms later
        let completed_at = started_at + 100_000_000; // 100ms later
        
        let metrics = TaskMetrics::new(&task, started_at, completed_at);
        assert_eq!(metrics.task_id, 1);
        assert_eq!(metrics.wait_time_ms, 50);
        assert_eq!(metrics.execution_time_ms, 100);
        assert_eq!(metrics.total_time_ms, 150);
    }
}
