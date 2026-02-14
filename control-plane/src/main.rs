use articos_scheduler::*;
use tracing::{info, Level};
use tracing_subscriber;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    ARTICOS Phase 1                         ║");
    println!("║              GPU Runtime Scheduler Simulation              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Run all simulation scenarios
    info!("Starting simulation scenarios...\n");
    
    scenario_1_fifo_basic();
    println!("\n{}\n", "═".repeat(60));
    
    scenario_2_fifo_varying_durations();
    println!("\n{}\n", "═".repeat(60));
    
    scenario_3_priority_basic();
    println!("\n{}\n", "═".repeat(60));
    
    scenario_4_priority_starvation();
    println!("\n{}\n", "═".repeat(60));
    
    scenario_5_resource_contention();
    
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║              Simulation Complete                           ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

/// Scenario 1: FIFO scheduler with uniform tasks
/// 
/// GPU analogy: Simple compute pipeline with equal-importance kernels
/// Expected: Fair scheduling, predictable execution order
fn scenario_1_fifo_basic() {
    info!("Scenario 1: FIFO Scheduler - Basic Uniform Tasks");
    info!("Simulating 10 tasks with equal priority and duration");
    info!("GPU equivalent: Sequential kernel launches on default stream\n");
    
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler); // 4 simulated SMs
    
    // Submit 10 uniform tasks
    for i in 0..10 {
        let task = Task::new(i, 0, 1, 100); // 100ms duration
        executor.submit_task(task);
    }
    
    info!("Submitted {} tasks to FIFO scheduler", 10);
    info!("Execution slots (SMs): {}", executor.total_slots());
    
    // Run simulation
    executor.run_until_complete(10000);
    
    // Print metrics
    executor.metrics().print_report();
}

/// Scenario 2: FIFO scheduler with varying task durations
/// 
/// GPU analogy: Mixed workload with short and long kernels
/// Expected: Head-of-line blocking - long tasks can delay short ones
fn scenario_2_fifo_varying_durations() {
    info!("Scenario 2: FIFO Scheduler - Varying Durations");
    info!("Simulating mixed workload: long and short tasks");
    info!("GPU equivalent: Mix of compute-heavy and lightweight kernels\n");
    
    let scheduler = Box::new(FifoScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit mix of long and short tasks
    let durations = vec![50, 200, 30, 300, 40, 150, 25, 250, 35, 100];
    for (i, duration) in durations.iter().enumerate() {
        let task = Task::new(i as u64, 0, 1, *duration);
        executor.submit_task(task);
    }
    
    info!("Submitted {} tasks with varying durations", durations.len());
    
    executor.run_until_complete(10000);
    executor.metrics().print_report();
}

/// Scenario 3: Priority scheduler with mixed priorities
/// 
/// GPU analogy: CUDA streams with different priorities
/// Expected: High-priority tasks execute before low-priority ones
fn scenario_3_priority_basic() {
    info!("Scenario 3: Priority Scheduler - Mixed Priorities");
    info!("Simulating tasks with high, medium, and low priorities");
    info!("GPU equivalent: CUDA high-priority streams for latency-sensitive work\n");
    
    let scheduler = Box::new(PriorityScheduler::new());
    let mut executor = Executor::new(4, scheduler);
    
    // Submit tasks with different priorities
    // Priority: 10=high, 5=medium, 1=low
    let tasks = vec![
        (0, 1, 100),   // Low priority
        (1, 10, 100),  // High priority
        (2, 5, 100),   // Medium priority
        (3, 1, 100),   // Low priority
        (4, 10, 100),  // High priority
        (5, 5, 100),   // Medium priority
        (6, 1, 100),   // Low priority
        (7, 10, 100),  // High priority
    ];
    
    for (id, priority, duration) in tasks {
        let task = Task::new(id, priority, 1, duration);
        executor.submit_task(task);
    }
    
    info!("Submitted {} tasks with priorities: Low=1, Med=5, High=10", 8);
    
    executor.run_until_complete(10000);
    executor.metrics().print_report();
    
    // Analyze by priority
    info!("Analyzing wait times by priority level:");
    let by_priority = executor.metrics().tasks_by_priority();
    for priority in [10, 5, 1] {
        if let Some(tasks) = by_priority.get(&priority) {
            let avg_wait: f64 = tasks.iter()
                .map(|t| t.wait_time_ms as f64)
                .sum::<f64>() / tasks.len() as f64;
            info!("  Priority {}: {:.2}ms avg wait", priority, avg_wait);
        }
    }
}

/// Scenario 4: Priority scheduler starvation demonstration
/// 
/// GPU analogy: Continuous high-priority stream starving background compute
/// Expected: Low-priority tasks experience severe starvation
fn scenario_4_priority_starvation() {
    info!("Scenario 4: Priority Scheduler - Starvation Scenario");
    info!("Simulating continuous high-priority tasks + few low-priority tasks");
    info!("GPU equivalent: Real-time rendering starving background compute\n");
    
    let scheduler = Box::new(PriorityScheduler::new());
    let mut executor = Executor::new(2, scheduler); // Only 2 SMs
    
    // Submit a few low-priority tasks first
    for i in 0..3 {
        let task = Task::new(i, 1, 1, 50);
        executor.submit_task(task);
    }
    
    // Then flood with high-priority tasks
    for i in 3..15 {
        let task = Task::new(i, 10, 1, 50);
        executor.submit_task(task);
    }
    
    info!("Submitted 3 low-priority + 12 high-priority tasks");
    info!("Execution slots (SMs): {} (resource constrained)", executor.total_slots());
    
    executor.run_until_complete(10000);
    executor.metrics().print_report();
    
    // Highlight starvation
    let agg = executor.metrics().aggregate();
    if agg.starved_tasks > 0 {
        info!("⚠️  STARVATION DETECTED: {} tasks experienced starvation", agg.starved_tasks);
        info!("   Max wait time: {}ms ({}x average execution time)", 
              agg.max_wait_time_ms, 
              agg.max_wait_time_ms as f64 / agg.avg_execution_time_ms);
    }
}

/// Scenario 5: Resource contention with limited SMs
/// 
/// GPU analogy: Multi-tenant GPU with resource limits
/// Expected: Shows impact of SM availability on throughput
fn scenario_5_resource_contention() {
    info!("Scenario 5: Resource Contention");
    info!("Comparing execution with different SM counts");
    info!("GPU equivalent: Comparing GPUs with different SM counts\n");
    
    let task_count = 20;
    let task_duration = 75;
    
    for sm_count in [2, 4, 8] {
        info!("--- Testing with {} SMs ---", sm_count);
        
        let scheduler = Box::new(FifoScheduler::new());
        let mut executor = Executor::new(sm_count, scheduler);
        
        for i in 0..task_count {
            let task = Task::new(i, 0, 1, task_duration);
            executor.submit_task(task);
        }
        
        executor.run_until_complete(10000);
        
        let agg = executor.metrics().aggregate();
        info!("  Avg wait time: {:.2}ms", agg.avg_wait_time_ms);
        info!("  Max wait time: {}ms", agg.max_wait_time_ms);
        info!("  Total simulation time: {}ms\n", agg.total_simulation_time_ms);
    }
}
