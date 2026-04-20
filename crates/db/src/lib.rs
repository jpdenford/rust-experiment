// use influxdb::{Client, WriteQuery};

pub fn add(left: u64, right: u64) -> u64 {
  left + right
}

// pub fn migrate(url: String, database: String) -> Result<String, MigrationFailure> {
//   let client = Client::new(url, database);
//   // client
//   // client.query(WriteQuery::new)
// }

#[cfg(test)]
#[allow(arithmetic_overflow)]
mod tests {
  use std::{
    f32::consts::PI,
    time::{SystemTime, UNIX_EPOCH},
  };

  use super::*;

  #[test]
  fn it_works() {
    let part = PI / 8.0;
    println!("{}", 0f32.sin());
    for i in 0..20 {
      let x = part * i as f32;
      println!("i: {}, x: {}, res: {}", i, x, x.sin());
    }
    // println!("{}", ().sin());
    // println!("{}", (PI / 2.0).sin());
    // println!("{}", PI.sin());
  }
}
