use articos_scheduler::*;

/// Integration test: Full execution cycle with FIFO scheduler
#[test]
fn test_fifo_full_execution() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(2, scheduler);
    
    // Submit 5 tasks
    for i in 0..5 {
        executor.submit_task(Task::new(i, 0, 1, 10));
    }
    
    assert_eq!(executor.pending_count(), 5);
    assert_eq!(executor.completed_count(), 0);
    
    // Run simulation
    executor.run_until_complete(10000);
    
    // Verify all tasks completed
    assert_eq!(executor.completed_count(), 5);
    assert_eq!(executor.pending_count(), 0);
    assert_eq!(executor.executing_count(), 0);
    
    // Verify metrics collected
    let metrics = executor.metrics();
    assert_eq!(metrics.task_metrics().len(), 5);
}

/// Integration test: Priority scheduler ordering
#[test]
fn test_priority_execution_order() {
    let scheduler = Box::new(PriorityScheduler::new());
    let mut executor = Executor::new(1, scheduler); // Single SM
    
    // Submit low then high priority
    executor.submit_task(Task::new(1, 1, 1, 10));  // Low
    executor.submit_task(Task::new(2, 10, 1, 10)); // High
    executor.submit_task(Task::new(3, 5, 1, 10));  // Med
    
    executor.run_until_complete(10000);
    
    // Verify all completed
    assert_eq!(executor.completed_count(), 3);
    
    // High priority should have lowest wait time
    let metrics = executor.metrics().task_metrics();
    let task2_wait = metrics.iter().find(|m| m.task_id == 2).unwrap().wait_time_ms;
    let task1_wait = metrics.iter().find(|m| m.task_id == 1).unwrap().wait_time_ms;
    
    // Task 2 (high priority) should have lower or equal wait time than task 1 (low)
    assert!(task2_wait <= task1_wait);
}

/// Integration test: Multiple execution slots
#[test]
fn test_parallel_execution() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler); // 4 SMs
    
    // Submit 8 tasks
    for i in 0..8 {
        executor.submit_task(Task::new(i, 0, 1, 50));
    }
    
    executor.run_until_complete(10000);
    
    assert_eq!(executor.completed_count(), 8);
    
    // With 4 slots, tasks should execute in parallel
    // Some tasks should have very low wait times
    let metrics = executor.metrics().task_metrics();
    let zero_wait_tasks = metrics.iter().filter(|m| m.wait_time_ms < 10).count();
    assert!(zero_wait_tasks >= 4, "At least 4 tasks should start immediately");
}

/// Integration test: Empty executor
#[test]
fn test_empty_executor() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(2, scheduler);
    
    // Run with no tasks
    executor.run_until_complete(100);
    
    assert_eq!(executor.completed_count(), 0);
    assert_eq!(executor.pending_count(), 0);
}

/// Integration test: Starvation detection
#[test]
fn test_starvation_detection() {
    let scheduler = Box::new(PriorityScheduler::new());
    let mut executor = Executor::new(1, scheduler); // Single SM
    
    // One low priority task
    executor.submit_task(Task::new(1, 1, 1, 10));
    
    // Many high priority tasks
    for i in 2..20 {
        executor.submit_task(Task::new(i, 10, 1, 10));
    }
    
    executor.run_until_complete(10000);
    
    let agg = executor.metrics().aggregate();
    // Low priority task should show signs of starvation
    assert!(agg.max_wait_time_ms as f64 > agg.avg_execution_time_ms * 5.0);
}

/// Integration test: Task metrics accuracy
#[test]
fn test_metrics_accuracy() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(1, scheduler);
    
    // Submit single task with known duration
    executor.submit_task(Task::new(1, 0, 1, 100));
    
    executor.run_until_complete(10000);
    
    let metrics = executor.metrics().task_metrics();
    assert_eq!(metrics.len(), 1);
    
    let task_metric = &metrics[0];
    assert_eq!(task_metric.task_id, 1);
    
    // Execution time should be close to 100ms (allow some variance)
    assert!(task_metric.execution_time_ms >= 90 && task_metric.execution_time_ms <= 110);
    
    // Total time should equal wait + execution
    assert_eq!(
        task_metric.total_time_ms,
        task_metric.wait_time_ms + task_metric.execution_time_ms
    );
}

/// Integration test: Resource slots management
#[test]
fn test_slot_management() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(3, scheduler);
    
    assert_eq!(executor.total_slots(), 3);
    assert_eq!(executor.available_slots(), 3);
    assert_eq!(executor.executing_count(), 0);
    
    // Submit 5 tasks
    for i in 0..5 {
        executor.submit_task(Task::new(i, 0, 1, 50));
    }
    
    executor.run_until_complete(10000);
    
    // After completion, all slots should be available
    assert_eq!(executor.available_slots(), 3);
    assert_eq!(executor.executing_count(), 0);
}

/// Integration test: Rapid task submission
#[test]
fn test_rapid_submission() {
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit many tasks rapidly
    for i in 0..100 {
        executor.submit_task(Task::new(i, 0, 1, 5)); // Short duration
    }
    
    assert_eq!(executor.pending_count(), 100);
    
    executor.run_until_complete(50000);
    
    assert_eq!(executor.completed_count(), 100);
    assert_eq!(executor.pending_count(), 0);
}

/// Integration test: Mixed scheduler behaviors
#[test]
fn test_fifo_vs_priority_fairness() {
    // FIFO should be more fair (lower variance in wait times)
    let fifo_scheduler = Box::new(FifoScheduler::new());
    let mut fifo_executor = Executor::new(2, fifo_scheduler);
    
    for i in 0..10 {
        let priority = if i % 2 == 0 { 10 } else { 1 };
        fifo_executor.submit_task(Task::new(i, priority, 1, 20));
    }
    
    fifo_executor.run_until_complete(10000);
    let fifo_agg = fifo_executor.metrics().aggregate();
    
    // Priority scheduler
    let priority_scheduler = Box::new(PriorityScheduler::new());
    let mut priority_executor = Executor::new(2, priority_scheduler);
    
    for i in 0..10 {
        let priority = if i % 2 == 0 { 10 } else { 1 };
        priority_executor.submit_task(Task::new(i, priority, 1, 20));
    }
    
    priority_executor.run_until_complete(10000);
    let priority_agg = priority_executor.metrics().aggregate();
    
    // Priority scheduler should have higher max wait time (unfairness)
    assert!(priority_agg.max_wait_time_ms >= fifo_agg.max_wait_time_ms);
}
