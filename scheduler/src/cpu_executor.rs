use crate::executor::TaskExecutor;
use crate::task::Task;
use std::time::Duration;

pub struct CpuExecutor;

impl TaskExecutor for CpuExecutor {
    fn execute(&mut self, task: &Task) -> anyhow::Result<()> {
        std::thread::sleep(Duration::from_millis(task.execution_time));
        Ok(())
    }
}