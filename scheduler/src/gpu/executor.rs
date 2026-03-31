use std::sync::{Once, OnceLock};

use cust::prelude::*;
use cust::memory::DeviceBuffer;
use cust::error::CudaResult;

use anyhow::Result;

use crate::task::Task;
use super::device::validate_device;
use super::context::create_context;

static CUDA_INIT: Once = Once::new();
static CUDA_CONTEXT: OnceLock<Context> = OnceLock::new();

pub struct GpuExecutor {
    _context: Context,
    streams: Vec<Stream>,
    current_stream: usize,
    module: Module,
}

impl GpuExecutor {
    pub fn new(num_streams: usize) -> Result<Self> {
        assert!(num_streams > 0);

        CUDA_INIT.call_once(|| {
            cust::init(cust::CudaFlags::empty()).expect("CUDA init failed");
        });

        let device = validate_device()?;

        let context = CUDA_CONTEXT
            .get_or_init(|| create_context(device).unwrap())
            .clone();

        let module = Module::from_ptx(
            include_str!("kernels/simulated_task.ptx"),
            &[],
        )?;

        let streams: Vec<Stream> = (0..num_streams)
            .map(|_| Stream::new(StreamFlags::NON_BLOCKING, None))
            .collect::<CudaResult<Vec<_>>>()?;

        Ok(Self {
            _context: context,
            streams,
            current_stream: 0,
            module,
        })
    }

    pub fn execute(&mut self, task: &Task) -> Result<()> {
        let iterations = (task.duration_ms as i32).saturating_mul(500_000);

        let blocks = (task.resource_requirement as u32) * 8;
        let threads_per_block = 256;

        let total_elements = (blocks * threads_per_block) as usize;

        let function = self.module.get_function("simulated_task")?;

        let buffer = DeviceBuffer::<f32>::zeroed(total_elements)?;

        let stream_index = self.current_stream;
        let stream = &self.streams[stream_index];

        self.current_stream = (self.current_stream + 1) % self.streams.len();

        unsafe {
            launch!(
                function<<<blocks, threads_per_block, 0, stream>>>(
                    buffer.as_device_ptr(),
                    iterations,
                    total_elements as i32
                )
            )?;
        }

        println!(
            "Task {} launched on GPU (stream {})",
            task.id, stream_index
        );

        Ok(())
    }

    pub fn synchronize_all(&self) {
        for stream in &self.streams {
            stream.synchronize().unwrap();
        }
    }
}