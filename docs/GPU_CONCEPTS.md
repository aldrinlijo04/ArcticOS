# GPU Scheduling Concepts - ARTICOS Background

This document explains the GPU hardware and software concepts that ARTICOS Phase 1 simulates.

## GPU Architecture Fundamentals

### Streaming Multiprocessors (SMs) / Compute Units (CUs)

**Hardware Reality:**
- Modern GPUs contain multiple execution units called SMs (NVIDIA) or CUs (AMD)
- Example: NVIDIA RTX 4090 has 128 SMs, AMD RX 7900 XTX has 96 CUs
- Each SM can execute multiple threads simultaneously (32-64 threads per warp/wavefront)
- SMs are the fundamental unit of parallel execution

**ARTICOS Simulation:**
- `Executor` slots represent individual SMs
- Each slot can execute one task at a time (simplified from real hardware)
- Fixed number of slots = fixed GPU compute capacity

### Warps and Wavefronts

**Hardware Reality:**
- GPU threads are grouped into warps (NVIDIA, 32 threads) or wavefronts (AMD, 64 threads)
- All threads in a warp execute the same instruction (SIMT: Single Instruction, Multiple Threads)
- Warps are scheduled onto SMs by the hardware scheduler
- No preemption within a warp - runs to completion or blocks on memory

**ARTICOS Simulation:**
- Tasks represent coarse-grained work (multiple warps)
- No preemption once scheduled (matches real GPU behavior)
- Duration simulates warp execution time

## GPU Scheduling Models

### Command Queues and Streams

**CUDA/HIP:**
```c
cudaStream_t stream;
cudaStreamCreate(&stream);
myKernel<<<grid, block, 0, stream>>>(args);  // Submit to stream
```

A stream is a sequence of operations that execute in order. Multiple streams can run concurrently.

**Vulkan:**
```c
VkQueue queue;
vkQueueSubmit(queue, 1, &submitInfo, fence);
```

Queues are hardware submission endpoints. Command buffers submitted to a queue execute in order.

**ARTICOS Simulation:**
- `Scheduler` represents the queue/stream management
- `submit()` is like kernel launch or queue submit
- `next()` is like hardware picking next command to execute

### Priority Levels

**CUDA Stream Priorities:**
```c
int priority;
cudaDeviceGetStreamPriorityRange(&leastPriority, &greatestPriority);
cudaStreamCreateWithPriority(&stream, cudaStreamDefault, priority);
```

Higher priority streams get preferential scheduling.

**Vulkan Queue Priorities:**
```c
float queuePriority = 1.0f;  // Range [0.0, 1.0]
VkDeviceQueueCreateInfo queueCreateInfo = {
    .queueFamilyIndex = queueFamily,
    .queueCount = 1,
    .pQueuePriorities = &queuePriority
};
```

**ARTICOS Simulation:**
- `PriorityScheduler` models stream/queue priorities
- Priority range 0-255 (0=lowest, 255=highest)
- Higher priority tasks scheduled before lower priority

## Scheduling Challenges

### 1. No Preemption

**Why GPUs Don't Preempt:**
- Context switching overhead is massive (thousands of threads)
- Shared memory state would need to be saved/restored
- Hardware designed for throughput, not latency
- Graphics workloads can't be interrupted mid-frame

**ARTICOS Simulation:**
- Once task assigned to execution slot, runs to completion
- Matches real GPU behavior
- Scheduling decisions only at task boundaries

### 2. Starvation

**Real-World Example:**
A high-priority graphics stream continuously submits rendering commands. A low-priority compute stream for background ML inference never gets scheduled.

**In Enterprise Systems:**
- Multi-tenant GPU clouds must prevent starvation
- Time-slicing (NVIDIA MIG, AMD partitioning) helps but adds overhead
- Fairness requires careful scheduling policies

**ARTICOS Simulation:**
- `PriorityScheduler` can exhibit starvation
- Metrics detect starvation (wait > 10× execution time)
- Real systems need mitigation (aging, quotas, fairness)

### 3. Head-of-Line Blocking

**Problem:**
A long-running kernel blocks all subsequent kernels in the same stream, even if they're short.

**Solution in Real GPUs:**
- Multiple execution contexts per SM
- Concurrent kernel execution (if resources allow)
- Multiple streams to avoid serialization

**ARTICOS Simulation:**
- `FifoScheduler` demonstrates head-of-line blocking
- Multiple execution slots allow parallel execution
- Shows importance of workload ordering

## Real GPU Scheduler Behavior

### NVIDIA GPU Scheduler

1. **Hardware Scheduling Units:**
   - GigaThread Engine manages work distribution
   - Each SM has a warp scheduler (1-4 per SM)
   - Picks warps from ready pool each cycle

2. **Stream Priorities:**
   - Implemented in software driver
   - Hardware agnostic - driver orders submissions
   - Priority affects command buffer submission order

3. **Concurrency:**
   - Multiple kernels can run simultaneously if resources allow
   - Limited by SM count, registers, shared memory
   - Dynamic resource allocation

### AMD GPU Scheduler

1. **Command Processor:**
   - Firmware-based scheduler
   - Manages multiple hardware queues
   - Submits wavefronts to compute units

2. **Queue Priorities:**
   - Hardware-supported queue priorities
   - Scheduling quantum per priority level
   - More integrated than NVIDIA approach

3. **Asynchronous Compute:**
   - Graphics and compute can overlap
   - Queue management critical for efficiency

## Metrics and Profiling

### What Real Profilers Measure

**NVIDIA Nsight:**
- Kernel execution time
- Queue wait time
- SM occupancy (% of theoretical maximum)
- Memory bandwidth utilization
- Warp stall reasons

**AMD rocProf:**
- Kernel duration
- Wavefront execution statistics
- Memory access patterns
- CU utilization

**ARTICOS Metrics:**
- Wait time (queue to execution)
- Execution time (simulated duration)
- Starvation detection
- Per-priority statistics

Maps directly to real profiler concepts!

## Phase 1 vs Real GPU

| Aspect | Real GPU | ARTICOS Phase 1 |
|--------|----------|-----------------|
| Execution Units | 10s-100s of SMs/CUs | Configurable slots (2-108) |
| Granularity | Warps/wavefronts (32-64 threads) | Coarse tasks |
| Preemption | None (within warp) | None |
| Scheduling | Hardware + driver | Software simulation |
| Concurrency | Thousands of threads | Multiple tasks |
| Resources | Registers, shared mem, L1/L2 | Simplified (future) |

## Future Phases

### Phase 2: Advanced Simulation
- Resource constraints (memory, bandwidth)
- Dynamic priority adjustment
- Preemption support (like NVIDIA compute preemption)
- Multi-level feedback queues

### Phase 3: Real GPU Integration
- CUDA driver API integration
- Measure real kernel execution
- Compare simulated vs actual behavior
- Production workload characterization

## References

1. **NVIDIA CUDA Programming Guide** - Section on streams and events
2. **AMD ROCm Documentation** - Queue and kernel scheduling
3. **Vulkan Specification** - Queue submission and synchronization
4. **"GPU Computing Gems"** - Chapter on scheduling optimization
5. **NVIDIA GTC Talks** - GPU architecture deep dives

## Key Takeaways

✅ GPU scheduling is **non-preemptive** at the warp/wavefront level  
✅ **Priorities** help latency-sensitive work but can cause starvation  
✅ **Multiple streams/queues** enable concurrency and avoid blocking  
✅ **Metrics collection** is essential for optimization  
✅ Real-world GPU schedulers are **complex** - many tradeoffs

ARTICOS Phase 1 captures these essential behaviors in a clean, testable simulation.
