use cust::context::Context;
use cust::device::Device;
use cust::error::CudaResult;

pub fn create_context(device: Device) -> CudaResult<Context> {
    Context::new(device)
}