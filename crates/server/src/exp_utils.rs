use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current time in millis
pub fn now_millis() -> u128 {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap() // okay to unwrap since we know it's after unix_epoch
    .as_millis();
  now
}
