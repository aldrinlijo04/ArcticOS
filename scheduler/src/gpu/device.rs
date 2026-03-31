use cust::device::Device;
use cust::error::CudaResult;

pub fn validate_device() -> CudaResult<Device> {
    let count = Device::num_devices()?;

    if count == 0 {
        panic!("No CUDA devices found");
    }

    let device = Device::get_device(0)?;

    println!("Using GPU: {}", device.name()?);

    Ok(device)
}