use articos_scheduler::{Executor, FifoScheduler, PriorityScheduler, Task};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
enum RunMode {
    Single,
    Autonomous,
    Nightly,
}

#[derive(Debug, Clone, Copy)]
enum SchedulerKind {
    Fifo,
    Priority,
}

#[derive(Debug, Clone, Copy)]
enum WorkloadKind {
    Uniform,
    Mixed,
    Bursty,
    PriorityFlood,
}

#[derive(Debug, Clone)]
struct CliConfig {
    mode: RunMode,
    scheduler: SchedulerKind,
    workload: WorkloadKind,
    slots: usize,
    tasks: u64,
    max_iterations: usize,
    seed: u64,
    output: String,
    baseline: Option<String>,
    history_db: String,
    report_md: String,
    report_json: String,
    schedulers: Vec<SchedulerKind>,
    workloads: Vec<WorkloadKind>,
    slots_list: Vec<usize>,
    seeds: Vec<u64>,
    ollama_model: Option<String>,
    ollama_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperimentConfig {
    scheduler: String,
    workload: String,
    slots: usize,
    tasks: u64,
    max_iterations: usize,
    seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriorityWaitStat {
    priority: u8,
    avg_wait_ms: f64,
    count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggregateSnapshot {
    total_tasks: usize,
    avg_wait_time_ms: f64,
    avg_execution_time_ms: f64,
    avg_total_time_ms: f64,
    max_wait_time_ms: u128,
    min_wait_time_ms: u128,
    starved_tasks: usize,
    total_simulation_time_ms: u128,
    throughput_tasks_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateDecision {
    gate: String,
    passed: bool,
    detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperimentArtifact {
    run_id: String,
    created_at_unix_nanos: u128,
    config: ExperimentConfig,
    aggregate: AggregateSnapshot,
    per_priority_wait: Vec<PriorityWaitStat>,
    gates: Vec<GateDecision>,
    insights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutonomousSummary {
    started_at_unix_nanos: u128,
    total_runs: usize,
    passed_runs: usize,
    failed_runs: usize,
    baseline_used: bool,
    best_run_id: Option<String>,
    best_score: Option<f64>,
    recommendation: String,
    top_runs: Vec<String>,
    llm_analysis: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock must be after UNIX_EPOCH")
        .as_nanos()
}

fn scheduler_name(kind: SchedulerKind) -> &'static str {
    match kind {
        SchedulerKind::Fifo => "fifo",
        SchedulerKind::Priority => "priority",
    }
}

fn workload_name(kind: WorkloadKind) -> &'static str {
    match kind {
        WorkloadKind::Uniform => "uniform",
        WorkloadKind::Mixed => "mixed",
        WorkloadKind::Bursty => "bursty",
        WorkloadKind::PriorityFlood => "priority-flood",
    }
}

fn parse_scheduler(value: &str) -> Result<SchedulerKind, String> {
    match value {
        "fifo" => Ok(SchedulerKind::Fifo),
        "priority" => Ok(SchedulerKind::Priority),
        other => Err(format!("Unsupported scheduler: {}", other)),
    }
}

fn parse_workload(value: &str) -> Result<WorkloadKind, String> {
    match value {
        "uniform" => Ok(WorkloadKind::Uniform),
        "mixed" => Ok(WorkloadKind::Mixed),
        "bursty" => Ok(WorkloadKind::Bursty),
        "priority-flood" => Ok(WorkloadKind::PriorityFlood),
        other => Err(format!("Unsupported workload: {}", other)),
    }
}

fn parse_scheduler_list(value: &str) -> Result<Vec<SchedulerKind>, String> {
    let mut out = Vec::new();
    for item in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        out.push(parse_scheduler(item)?);
    }
    if out.is_empty() {
        return Err("--schedulers must contain at least one entry".to_string());
    }
    Ok(out)
}

fn parse_workload_list(value: &str) -> Result<Vec<WorkloadKind>, String> {
    let mut out = Vec::new();
    for item in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        out.push(parse_workload(item)?);
    }
    if out.is_empty() {
        return Err("--workloads must contain at least one entry".to_string());
    }
    Ok(out)
}

fn parse_usize_list(value: &str, name: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for item in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let parsed: usize = item
            .parse()
            .map_err(|_| format!("Invalid value '{}' in {}", item, name))?;
        out.push(parsed);
    }
    if out.is_empty() {
        return Err(format!("{} must contain at least one entry", name));
    }
    Ok(out)
}

fn parse_u64_list(value: &str, name: &str) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    for item in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let parsed: u64 = item
            .parse()
            .map_err(|_| format!("Invalid value '{}' in {}", item, name))?;
        out.push(parsed);
    }
    if out.is_empty() {
        return Err(format!("{} must contain at least one entry", name));
    }
    Ok(out)
}

fn parse_args() -> Result<CliConfig, String> {
    let mut mode = RunMode::Single;
    let mut scheduler = SchedulerKind::Fifo;
    let mut workload = WorkloadKind::Uniform;
    let mut slots = 4usize;
    let mut tasks = 50u64;
    let mut max_iterations = 200_000usize;
    let mut seed = 42u64;
    let mut output = String::from("artifacts/latest_run.json");
    let mut baseline: Option<String> = None;
    let mut history_db = String::from("artifacts/agentic_runs.db");
    let mut report_md = String::from("artifacts/autonomous_report.md");
    let mut report_json = String::from("artifacts/autonomous_report.json");
    let mut schedulers = vec![SchedulerKind::Fifo, SchedulerKind::Priority];
    let mut workloads = vec![
        WorkloadKind::Uniform,
        WorkloadKind::Mixed,
        WorkloadKind::Bursty,
        WorkloadKind::PriorityFlood,
    ];
    let mut slots_list = vec![2usize, 4usize, 8usize];
    let mut seeds = vec![41u64, 42u64, 43u64];
    let mut ollama_model: Option<String> = None;
    let mut ollama_url = String::from("http://127.0.0.1:11434/api/generate");

    let args: Vec<String> = env::args().collect();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                let value = args.get(i).ok_or("Missing value for --mode")?;
                mode = match value.as_str() {
                    "single" => RunMode::Single,
                    "autonomous" => RunMode::Autonomous,
                    "nightly" => RunMode::Nightly,
                    other => return Err(format!("Unsupported mode: {}", other)),
                };
            }
            "--scheduler" => {
                i += 1;
                scheduler = parse_scheduler(args.get(i).ok_or("Missing value for --scheduler")?)?;
            }
            "--workload" => {
                i += 1;
                workload = parse_workload(args.get(i).ok_or("Missing value for --workload")?)?;
            }
            "--slots" => {
                i += 1;
                slots = args
                    .get(i)
                    .ok_or("Missing value for --slots")?
                    .parse()
                    .map_err(|_| "Invalid --slots value")?;
            }
            "--tasks" => {
                i += 1;
                tasks = args
                    .get(i)
                    .ok_or("Missing value for --tasks")?
                    .parse()
                    .map_err(|_| "Invalid --tasks value")?;
            }
            "--max-iterations" => {
                i += 1;
                max_iterations = args
                    .get(i)
                    .ok_or("Missing value for --max-iterations")?
                    .parse()
                    .map_err(|_| "Invalid --max-iterations value")?;
            }
            "--seed" => {
                i += 1;
                seed = args
                    .get(i)
                    .ok_or("Missing value for --seed")?
                    .parse()
                    .map_err(|_| "Invalid --seed value")?;
            }
            "--output" => {
                i += 1;
                output = args.get(i).ok_or("Missing value for --output")?.to_string();
            }
            "--baseline" => {
                i += 1;
                baseline = Some(args.get(i).ok_or("Missing value for --baseline")?.to_string());
            }
            "--history-db" => {
                i += 1;
                history_db = args
                    .get(i)
                    .ok_or("Missing value for --history-db")?
                    .to_string();
            }
            "--report-md" => {
                i += 1;
                report_md = args
                    .get(i)
                    .ok_or("Missing value for --report-md")?
                    .to_string();
            }
            "--report-json" => {
                i += 1;
                report_json = args
                    .get(i)
                    .ok_or("Missing value for --report-json")?
                    .to_string();
            }
            "--schedulers" => {
                i += 1;
                schedulers = parse_scheduler_list(args.get(i).ok_or("Missing value for --schedulers")?)?;
            }
            "--workloads" => {
                i += 1;
                workloads = parse_workload_list(args.get(i).ok_or("Missing value for --workloads")?)?;
            }
            "--slots-list" => {
                i += 1;
                slots_list = parse_usize_list(args.get(i).ok_or("Missing value for --slots-list")?, "--slots-list")?;
            }
            "--seeds" => {
                i += 1;
                seeds = parse_u64_list(args.get(i).ok_or("Missing value for --seeds")?, "--seeds")?;
            }
            "--ollama-model" => {
                i += 1;
                ollama_model = Some(args.get(i).ok_or("Missing value for --ollama-model")?.to_string());
            }
            "--ollama-url" => {
                i += 1;
                ollama_url = args
                    .get(i)
                    .ok_or("Missing value for --ollama-url")?
                    .to_string();
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("Unknown argument: {}", other)),
        }
        i += 1;
    }

    if slots == 0 {
        return Err("--slots must be >= 1".to_string());
    }
    if tasks == 0 {
        return Err("--tasks must be >= 1".to_string());
    }

    Ok(CliConfig {
        mode,
        scheduler,
        workload,
        slots,
        tasks,
        max_iterations,
        seed,
        output,
        baseline,
        history_db,
        report_md,
        report_json,
        schedulers,
        workloads,
        slots_list,
        seeds,
        ollama_model,
        ollama_url,
    })
}

fn print_help() {
    println!("articos-agentic - autonomous local experiment orchestrator");
    println!("");
    println!("Usage:");
    println!("  cargo run -p articos-agentic -- [options]");
    println!("");
    println!("Core Options:");
    println!("  --mode <single|autonomous|nightly>    Run mode (default: single)");
    println!("  --scheduler <fifo|priority>           Scheduler policy (single mode)");
    println!("  --workload <uniform|mixed|bursty|priority-flood>  Workload (single mode)");
    println!("  --slots <N>                           Slots (single mode)");
    println!("  --tasks <N>                           Number of tasks (default: 50)");
    println!("  --max-iterations <N>                  Loop cap (default: 200000)");
    println!("  --seed <N>                            Seed (single mode)");
    println!("  --output <path>                       Artifact output path");
    println!("  --baseline <path>                     Baseline artifact for gate comparison");
    println!("");
    println!("Autonomous Mode Options:");
    println!("  --schedulers <csv>                    e.g. fifo,priority");
    println!("  --workloads <csv>                     e.g. uniform,mixed,bursty,priority-flood");
    println!("  --slots-list <csv>                    e.g. 2,4,8");
    println!("  --seeds <csv>                         e.g. 41,42,43");
    println!("  --history-db <path>                   SQLite history DB path");
    println!("  --report-md <path>                    Markdown report path");
    println!("  --report-json <path>                  JSON summary path");
    println!("  --ollama-model <name>                 Optional local Ollama model");
    println!("  --ollama-url <url>                    Ollama generate endpoint URL");
}

fn lcg_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    *seed
}

fn gen_tasks(workload: WorkloadKind, count: u64, mut seed: u64) -> Vec<Task> {
    let mut tasks = Vec::with_capacity(count as usize);

    for id in 0..count {
        let task = match workload {
            WorkloadKind::Uniform => Task::new(id, 0, 1, 100),
            WorkloadKind::Mixed => {
                let r = lcg_next(&mut seed);
                let duration = 20 + (r % 181);
                let priority = (lcg_next(&mut seed) % 11) as u8;
                Task::new(id, priority, 1, duration)
            }
            WorkloadKind::Bursty => {
                let phase = (id / 10) % 3;
                match phase {
                    0 => Task::new(id, 2, 1, 30),
                    1 => Task::new(id, 5, 1, 200),
                    _ => Task::new(id, 8, 1, 60),
                }
            }
            WorkloadKind::PriorityFlood => {
                if id < count / 5 {
                    Task::new(id, 1, 1, 50)
                } else {
                    Task::new(id, 10, 1, 50)
                }
            }
        };
        tasks.push(task);
    }

    tasks
}

fn build_snapshot(executor: &Executor) -> (AggregateSnapshot, Vec<PriorityWaitStat>) {
    let agg = executor.metrics().aggregate();
    let elapsed_secs = if agg.total_simulation_time_ms == 0 {
        0.0
    } else {
        agg.total_simulation_time_ms as f64 / 1000.0
    };
    let throughput = if elapsed_secs == 0.0 {
        0.0
    } else {
        agg.total_tasks as f64 / elapsed_secs
    };

    let mut by_priority: HashMap<u8, Vec<f64>> = HashMap::new();
    for item in executor.metrics().task_metrics() {
        by_priority
            .entry(item.priority)
            .or_default()
            .push(item.wait_time_ms as f64);
    }

    let mut stats: Vec<PriorityWaitStat> = by_priority
        .into_iter()
        .map(|(priority, waits)| {
            let sum: f64 = waits.iter().sum();
            let avg = if waits.is_empty() {
                0.0
            } else {
                sum / waits.len() as f64
            };
            PriorityWaitStat {
                priority,
                avg_wait_ms: avg,
                count: waits.len(),
            }
        })
        .collect();
    stats.sort_by_key(|s| s.priority);

    (
        AggregateSnapshot {
            total_tasks: agg.total_tasks,
            avg_wait_time_ms: agg.avg_wait_time_ms,
            avg_execution_time_ms: agg.avg_execution_time_ms,
            avg_total_time_ms: agg.avg_total_time_ms,
            max_wait_time_ms: agg.max_wait_time_ms,
            min_wait_time_ms: agg.min_wait_time_ms,
            starved_tasks: agg.starved_tasks,
            total_simulation_time_ms: agg.total_simulation_time_ms,
            throughput_tasks_per_sec: throughput,
        },
        stats,
    )
}

fn evaluate_gates(candidate: &AggregateSnapshot, baseline: Option<&AggregateSnapshot>) -> Vec<GateDecision> {
    let mut gates = Vec::new();
    gates.push(GateDecision {
        gate: "Gate A: Valid Run Artifact".to_string(),
        passed: true,
        detail: "Run completed and metrics snapshot was generated".to_string(),
    });

    if let Some(base) = baseline {
        let starvation_pass = candidate.starved_tasks <= base.starved_tasks;
        gates.push(GateDecision {
            gate: "Gate B: Starvation Non-Increase".to_string(),
            passed: starvation_pass,
            detail: format!(
                "candidate={} baseline={} (must be <=)",
                candidate.starved_tasks, base.starved_tasks
            ),
        });

        let max_wait_cap = (base.max_wait_time_ms as f64 * 1.15).round() as u128;
        gates.push(GateDecision {
            gate: "Gate C: Max Wait <= 1.15x Baseline".to_string(),
            passed: candidate.max_wait_time_ms <= max_wait_cap,
            detail: format!(
                "candidate={} baseline_cap={}",
                candidate.max_wait_time_ms, max_wait_cap
            ),
        });

        let throughput_floor = base.throughput_tasks_per_sec * 0.95;
        gates.push(GateDecision {
            gate: "Gate D: Throughput >= 0.95x Baseline".to_string(),
            passed: candidate.throughput_tasks_per_sec >= throughput_floor,
            detail: format!(
                "candidate={:.3} baseline_floor={:.3}",
                candidate.throughput_tasks_per_sec, throughput_floor
            ),
        });
    } else {
        gates.push(GateDecision {
            gate: "Gate B/C/D: Baseline Comparison".to_string(),
            passed: true,
            detail: "No baseline provided; comparison gates skipped".to_string(),
        });
    }

    gates.push(GateDecision {
        gate: "Gate E: Recommendation-Only Mode".to_string(),
        passed: true,
        detail: "Autonomous layer only writes artifacts/reports and never mutates scheduler code"
            .to_string(),
    });

    gates
}

fn derive_insights(
    snapshot: &AggregateSnapshot,
    per_priority: &[PriorityWaitStat],
    gates: &[GateDecision],
) -> Vec<String> {
    let mut insights = Vec::new();
    insights.push(format!(
        "Throughput: {:.3} tasks/sec across {} tasks",
        snapshot.throughput_tasks_per_sec, snapshot.total_tasks
    ));
    insights.push(format!(
        "Wait profile: avg={:.2}ms max={}ms starved={}",
        snapshot.avg_wait_time_ms, snapshot.max_wait_time_ms, snapshot.starved_tasks
    ));

    if let (Some(low), Some(high)) = (per_priority.first(), per_priority.last()) {
        insights.push(format!(
            "Priority spread: p{} avg_wait={:.2}ms -> p{} avg_wait={:.2}ms",
            low.priority, low.avg_wait_ms, high.priority, high.avg_wait_ms
        ));
    }

    let failed: Vec<&GateDecision> = gates.iter().filter(|g| !g.passed).collect();
    if failed.is_empty() {
        insights.push("All active quality gates passed".to_string());
    } else {
        insights.push(format!(
            "Failed gates: {}",
            failed
                .iter()
                .map(|g| g.gate.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    insights
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir for '{}': {}", path, e))?;
        }
    }
    Ok(())
}

fn run_experiment(cfg: &ExperimentConfig, baseline: Option<&AggregateSnapshot>) -> ExperimentArtifact {
    let mut executor = match cfg.scheduler.as_str() {
        "fifo" => Executor::new(cfg.slots, Box::new(FifoScheduler::new())),
        "priority" => Executor::new(cfg.slots, Box::new(PriorityScheduler::new())),
        _ => Executor::new(cfg.slots, Box::new(FifoScheduler::new())),
    };

    let workload = parse_workload(&cfg.workload).unwrap_or(WorkloadKind::Uniform);
    let tasks = gen_tasks(workload, cfg.tasks, cfg.seed);
    for task in tasks {
        executor.submit_task(task);
    }
    executor.run_until_complete(cfg.max_iterations);

    let (aggregate, per_priority_wait) = build_snapshot(&executor);
    let gates = evaluate_gates(&aggregate, baseline);
    let insights = derive_insights(&aggregate, &per_priority_wait, &gates);
    let created_at = now_nanos();
    let run_id = format!(
        "run-{}-{}-{}-s{}-seed{}",
        created_at, cfg.scheduler, cfg.workload, cfg.slots, cfg.seed
    );

    ExperimentArtifact {
        run_id,
        created_at_unix_nanos: created_at,
        config: cfg.clone(),
        aggregate,
        per_priority_wait,
        gates,
        insights,
    }
}

fn read_baseline(path: Option<&String>) -> Result<Option<AggregateSnapshot>, String> {
    if let Some(p) = path {
        let raw = fs::read_to_string(p)
            .map_err(|e| format!("Failed to read baseline artifact '{}': {}", p, e))?;
        let parsed: ExperimentArtifact = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse baseline artifact '{}': {}", p, e))?;
        Ok(Some(parsed.aggregate))
    } else {
        Ok(None)
    }
}

fn store_artifact(conn: &Connection, artifact: &ExperimentArtifact) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            created_at_unix_nanos TEXT NOT NULL,
            scheduler TEXT NOT NULL,
            workload TEXT NOT NULL,
            slots INTEGER NOT NULL,
            tasks INTEGER NOT NULL,
            seed INTEGER NOT NULL,
            throughput REAL NOT NULL,
            avg_wait REAL NOT NULL,
            max_wait TEXT NOT NULL,
            starved INTEGER NOT NULL,
            passed INTEGER NOT NULL,
            artifact_json TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("Failed to initialize run history table: {}", e))?;

    let passed = if artifact.gates.iter().all(|g| g.passed) {
        1
    } else {
        0
    };
    let json = serde_json::to_string(artifact)
        .map_err(|e| format!("Failed to serialize artifact for DB: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO runs (
            run_id, created_at_unix_nanos, scheduler, workload, slots, tasks, seed,
            throughput, avg_wait, max_wait, starved, passed, artifact_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            artifact.run_id,
            artifact.created_at_unix_nanos.to_string(),
            artifact.config.scheduler,
            artifact.config.workload,
            artifact.config.slots as i64,
            artifact.config.tasks as i64,
            artifact.config.seed as i64,
            artifact.aggregate.throughput_tasks_per_sec,
            artifact.aggregate.avg_wait_time_ms,
            artifact.aggregate.max_wait_time_ms.to_string(),
            artifact.aggregate.starved_tasks as i64,
            passed,
            json,
        ],
    )
    .map_err(|e| format!("Failed to store run '{}' in DB: {}", artifact.run_id, e))?;

    Ok(())
}

fn score_artifact(artifact: &ExperimentArtifact, baseline: Option<&AggregateSnapshot>) -> f64 {
    if let Some(base) = baseline {
        let thr = if base.throughput_tasks_per_sec <= 0.0 {
            artifact.aggregate.throughput_tasks_per_sec
        } else {
            artifact.aggregate.throughput_tasks_per_sec / base.throughput_tasks_per_sec
        };
        let wait_penalty = if base.avg_wait_time_ms <= 0.0 {
            artifact.aggregate.avg_wait_time_ms / 1000.0
        } else {
            artifact.aggregate.avg_wait_time_ms / base.avg_wait_time_ms
        };
        let starve_penalty = artifact.aggregate.starved_tasks as f64 * 0.5;
        thr - (0.45 * wait_penalty) - starve_penalty
    } else {
        artifact.aggregate.throughput_tasks_per_sec
            - (artifact.aggregate.avg_wait_time_ms * 0.1)
            - (artifact.aggregate.starved_tasks as f64 * 20.0)
    }
}

fn try_ollama_analysis(
    model: &str,
    url: &str,
    summary: &AutonomousSummary,
    best_artifact: Option<&ExperimentArtifact>,
) -> Option<String> {
    let best = best_artifact.map(|a| {
        format!(
            "best_run={} scheduler={} workload={} slots={} seed={} throughput={:.3} avg_wait={:.2} starved={}",
            a.run_id,
            a.config.scheduler,
            a.config.workload,
            a.config.slots,
            a.config.seed,
            a.aggregate.throughput_tasks_per_sec,
            a.aggregate.avg_wait_time_ms,
            a.aggregate.starved_tasks
        )
    });

    let prompt = format!(
        "You are analyzing scheduler experiments for ARCTICOS.\nTotal runs: {}\nPassed runs: {}\nFailed runs: {}\nRecommendation: {}\n{}\nGive concise advice with 3 next steps.",
        summary.total_runs,
        summary.passed_runs,
        summary.failed_runs,
        summary.recommendation,
        best.unwrap_or_else(|| "best_run=none".to_string())
    );

    let body = OllamaGenerateRequest {
        model,
        prompt: &prompt,
        stream: false,
    };

    let client = Client::builder().build().ok()?;
    let response = client.post(url).json(&body).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: OllamaGenerateResponse = response.json().ok()?;
    Some(parsed.response.trim().to_string())
}

fn write_markdown_report(
    path: &str,
    summary: &AutonomousSummary,
    artifacts: &[ExperimentArtifact],
    best: Option<&ExperimentArtifact>,
) -> Result<(), String> {
    ensure_parent_dir(path)?;

    let mut lines = Vec::new();
    lines.push("# ARCTICOS Autonomous Agent Report".to_string());
    lines.push("".to_string());
    lines.push(format!("- Total runs: {}", summary.total_runs));
    lines.push(format!("- Passed runs: {}", summary.passed_runs));
    lines.push(format!("- Failed runs: {}", summary.failed_runs));
    lines.push(format!("- Baseline used: {}", summary.baseline_used));
    lines.push(format!("- Recommendation: {}", summary.recommendation));
    lines.push("".to_string());

    if let Some(best_run) = best {
        lines.push("## Best Candidate".to_string());
        lines.push(format!("- Run ID: {}", best_run.run_id));
        lines.push(format!("- Scheduler: {}", best_run.config.scheduler));
        lines.push(format!("- Workload: {}", best_run.config.workload));
        lines.push(format!("- Slots: {}", best_run.config.slots));
        lines.push(format!("- Seed: {}", best_run.config.seed));
        lines.push(format!(
            "- Throughput: {:.3} tasks/sec",
            best_run.aggregate.throughput_tasks_per_sec
        ));
        lines.push(format!("- Avg Wait: {:.2} ms", best_run.aggregate.avg_wait_time_ms));
        lines.push(format!("- Starved tasks: {}", best_run.aggregate.starved_tasks));
        lines.push("".to_string());
    }

    lines.push("## Top Runs".to_string());
    for run in artifacts.iter().take(5) {
        let gate_status = if run.gates.iter().all(|g| g.passed) {
            "PASS"
        } else {
            "FAIL"
        };
        lines.push(format!(
            "- {} | {} | {} | slots={} seed={} | thr={:.3} | wait={:.2} | starved={} | {}",
            run.run_id,
            run.config.scheduler,
            run.config.workload,
            run.config.slots,
            run.config.seed,
            run.aggregate.throughput_tasks_per_sec,
            run.aggregate.avg_wait_time_ms,
            run.aggregate.starved_tasks,
            gate_status
        ));
    }

    if let Some(text) = summary.llm_analysis.as_ref() {
        lines.push("".to_string());
        lines.push("## Optional Local LLM Analysis".to_string());
        lines.push(text.to_string());
    }

    fs::write(path, lines.join("\n"))
        .map_err(|e| format!("Failed to write markdown report '{}': {}", path, e))?;
    Ok(())
}

fn write_json<T: Serialize>(path: &str, value: &T) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize JSON for '{}': {}", path, e))?;
    fs::write(path, content).map_err(|e| format!("Failed to write '{}': {}", path, e))
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

fn run_single(cfg: &CliConfig) -> Result<(), String> {
    let baseline = read_baseline(cfg.baseline.as_ref())?;
    let exp_cfg = ExperimentConfig {
        scheduler: scheduler_name(cfg.scheduler).to_string(),
        workload: workload_name(cfg.workload).to_string(),
        slots: cfg.slots,
        tasks: cfg.tasks,
        max_iterations: cfg.max_iterations,
        seed: cfg.seed,
    };
    let artifact = run_experiment(&exp_cfg, baseline.as_ref());
    write_json(&cfg.output, &artifact)?;

    println!("Experiment completed: {}", artifact.run_id);
    println!("Scheduler: {}", artifact.config.scheduler);
    println!("Workload: {}", artifact.config.workload);
    println!("Slots: {}", artifact.config.slots);
    println!("Tasks: {}", artifact.config.tasks);
    println!(
        "Throughput: {:.3} tasks/sec",
        artifact.aggregate.throughput_tasks_per_sec
    );
    println!("Starved tasks: {}", artifact.aggregate.starved_tasks);
    println!("Artifact: {}", cfg.output);

    let failed = artifact.gates.iter().filter(|g| !g.passed).count();
    if failed > 0 {
        println!("Gate status: {} failed", failed);
    } else {
        println!("Gate status: all passed");
    }
    Ok(())
}

fn run_autonomous(cfg: &CliConfig) -> Result<(), String> {
    let baseline = read_baseline(cfg.baseline.as_ref())?;
    ensure_parent_dir(&cfg.history_db)?;
    let conn = Connection::open(&cfg.history_db)
        .map_err(|e| format!("Failed to open history DB '{}': {}", cfg.history_db, e))?;

    let mut artifacts = Vec::new();
    for scheduler in &cfg.schedulers {
        for workload in &cfg.workloads {
            for slots in &cfg.slots_list {
                for seed in &cfg.seeds {
                    let exp_cfg = ExperimentConfig {
                        scheduler: scheduler_name(*scheduler).to_string(),
                        workload: workload_name(*workload).to_string(),
                        slots: *slots,
                        tasks: cfg.tasks,
                        max_iterations: cfg.max_iterations,
                        seed: *seed,
                    };
                    let artifact = run_experiment(&exp_cfg, baseline.as_ref());
                    store_artifact(&conn, &artifact)?;
                    artifacts.push(artifact);
                }
            }
        }
    }

    artifacts.sort_by(|a, b| {
        let sa = score_artifact(a, baseline.as_ref());
        let sb = score_artifact(b, baseline.as_ref());
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let passed_runs = artifacts
        .iter()
        .filter(|a| a.gates.iter().all(|g| g.passed))
        .count();
    let failed_runs = artifacts.len().saturating_sub(passed_runs);

    let best_artifact = artifacts
        .iter()
        .find(|a| a.gates.iter().all(|g| g.passed))
        .or_else(|| artifacts.first());

    let recommendation = if let Some(best) = best_artifact {
        if best.gates.iter().all(|g| g.passed) {
            format!(
                "Promote candidate '{}' ({}/{}, slots={}, seed={})",
                best.run_id, best.config.scheduler, best.config.workload, best.config.slots, best.config.seed
            )
        } else {
            "No candidate passed all gates. Keep current baseline policy.".to_string()
        }
    } else {
        "No runs were produced; check CLI parameters".to_string()
    };

    let mut summary = AutonomousSummary {
        started_at_unix_nanos: now_nanos(),
        total_runs: artifacts.len(),
        passed_runs,
        failed_runs,
        baseline_used: baseline.is_some(),
        best_run_id: best_artifact.map(|a| a.run_id.clone()),
        best_score: best_artifact.map(|a| score_artifact(a, baseline.as_ref())),
        recommendation,
        top_runs: artifacts.iter().take(5).map(|a| a.run_id.clone()).collect(),
        llm_analysis: None,
    };

    if let Some(model) = cfg.ollama_model.as_ref() {
        summary.llm_analysis = try_ollama_analysis(model, &cfg.ollama_url, &summary, best_artifact);
    }

    if let Some(best) = best_artifact {
        write_json(&cfg.output, best)?;
    }
    write_json(&cfg.report_json, &summary)?;
    write_markdown_report(&cfg.report_md, &summary, &artifacts, best_artifact)?;

    println!("Autonomous run complete");
    println!("Total runs: {}", summary.total_runs);
    println!("Passed: {}", summary.passed_runs);
    println!("Failed: {}", summary.failed_runs);
    println!("Recommendation: {}", summary.recommendation);
    println!("History DB: {}", cfg.history_db);
    println!("Best artifact: {}", cfg.output);
    println!("JSON summary: {}", cfg.report_json);
    println!("Markdown report: {}", cfg.report_md);

    Ok(())
}

fn run_nightly(cfg: &CliConfig) -> Result<(), String> {
    run_autonomous(cfg)?;

    let raw = fs::read_to_string(&cfg.report_json).map_err(|e| {
        format!(
            "Nightly check could not read summary '{}': {}",
            cfg.report_json, e
        )
    })?;
    let summary: AutonomousSummary = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "Nightly check could not parse summary '{}': {}",
            cfg.report_json, e
        )
    })?;

    // Nightly policy: fail only if no candidate passed, or best recommendation is absent.
    if summary.passed_runs == 0 || summary.best_run_id.is_none() {
        return Err(format!(
            "Nightly regression failed: passed_runs={} best_run_id={:?}",
            summary.passed_runs, summary.best_run_id
        ));
    }

    println!(
        "Nightly regression status: PASS (passed_runs={} total_runs={})",
        summary.passed_runs, summary.total_runs
    );
    Ok(())
}

fn run() -> Result<(), String> {
    let cfg = parse_args()?;
    match cfg.mode {
        RunMode::Single => run_single(&cfg),
        RunMode::Autonomous => run_autonomous(&cfg),
        RunMode::Nightly => run_nightly(&cfg),
    }
}
