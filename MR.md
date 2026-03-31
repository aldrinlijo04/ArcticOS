## 🚀 GPU Execution Integration

### Summary
This MR adds GPU-backed task execution to the ARCTICOS scheduler using CUDA.

### Changes
- Added `GpuExecutor` using Rust `cust` crate
- Implemented multi-stream execution (parallel GPU scheduling)
- Integrated GPU execution into runtime executor
- Added asynchronous kernel dispatch
- Added final GPU synchronization to ensure completion

### Architecture
Scheduler → Executor → GpuExecutor → CUDA Streams → Kernel

### Impact
- Enables real GPU execution of tasks
- Simulates parallel execution using CUDA streams
- Improves realism of scheduler behavior

### Limitations
- Uses synthetic GPU workloads (`simulated_task`)
- Parallelism is limited by kernel design

### Notes
Tested on NVIDIA RTX 3050 with CUDA support.