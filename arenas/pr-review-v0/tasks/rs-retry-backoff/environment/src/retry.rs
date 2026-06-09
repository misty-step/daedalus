use std::thread::sleep;
use std::time::Duration;

/// Retry `op` up to `max_attempts` times with exponential backoff.
pub fn retry<T, E, F>(mut op: F, max_attempts: u32) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut attempt = 0;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if attempt > max_attempts {
                    return Err(e);
                }
                sleep(Duration::from_millis(100 * 2u64.pow(attempt)));
            }
        }
    }
}
