use crate::config::Args;
use crate::ingestion::run_ingestion;
use clap::Parser;
use influxdb::{Client, ReadQuery};
use serde_json::Value;
use std::num::ParseIntError;
use std::time::Duration;
use tokio::select;
use tokio::time::sleep;
mod config;
mod exp_utils;
mod ingestion;
mod model;
mod persistence;
mod simulation;

#[tokio::main]
async fn main() {
  let config = Args::parse();

  let pipeline = run_ingestion(config.clone());
  // Note that a stream may or may not be the best choice to simulate the
  // Live configuration. If live, there is a separate process capturing samples, then
  // we'd likely want to use some kind of buffered mechanism (e.g. channel?)

  // Run the experiment for the duration specified
  let timer = sleep(Duration::from_secs(config.duration.into()));
  select! {
    _pipe = pipeline => {
      println!("Pipeline stopped?!")
    },
    _timer = timer => {
      println!("Experiment end reached!")
    }
  }

  let db_client = Client::new(config.db_host, config.db_database).with_token(config.db_token);

  let res = get_table_count(db_client, "readings").await;

  match res {
    Ok(count) => println!("Row Count: {}", count),
    Err(err) => println!("Failed to get row count: {:?}", err),
  }
}

#[derive(Debug)]
enum CountError {
  Db(influxdb::Error),
  ResponseShape(serde_json::Error),
  NumParse(ParseIntError),
}

async fn get_table_count(db_client: Client, table_name: &str) -> Result<u64, CountError> {
  db_client
    .query(ReadQuery::new(format!(
      "SELECT count(*) from {}",
      table_name
    )))
    .await
    .map_err(CountError::Db)
    .and_then(|r| {
      let v: Value = serde_json::from_str(r.as_str()).map_err(CountError::ResponseShape)?;
      let count = &v["results"][0]["series"][0]["values"][0][1];
      Ok(count.to_string())
    })
    .and_then(|count_string| count_string.parse::<u64>().map_err(CountError::NumParse))
}
