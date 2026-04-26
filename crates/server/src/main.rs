use crate::config::Args;
use crate::ingestion::{IngestionConfig, run_ingestion};
use crate::persistence::fetch_measurement_count;
use clap::Parser;
use influxdb::Client;
use std::time::Duration;
use tokio::select;
use tokio::time::sleep;
mod config;
mod core;
mod ingestion;
mod persistence;
mod simulation;

#[tokio::main]
async fn main() {
  let config = Args::parse();

  let ingestion_config: IngestionConfig = config.clone().into();
  let db_client = Client::new(config.db_host, config.db_database).with_token(config.db_token);

  let pipeline = run_ingestion(ingestion_config, db_client.clone());

  // Run the experiment for the duration specified. Note: could use cancellation token here
  let timer = sleep(Duration::from_secs(config.duration.into()));
  select! {
    _ = pipeline => println!("Pipeline stopped!"),
    _ = timer => println!("Experiment end reached!")
  }

  let res = fetch_measurement_count(db_client, "readings").await;

  match res {
    Ok(count) => println!("Row Count: {}", count),
    Err(err) => println!("Failed to get row count: {:?}", err),
  }
}

impl From<Args> for IngestionConfig {
  fn from(value: Args) -> Self {
    match value.command {
      config::SamplingMode::Live { .. } => todo!("Not yet implemented"),
      config::SamplingMode::Simulated {
        num_sensors,
        sample_rate_ms,
      } => IngestionConfig::Simulated {
        num_sensors,
        sample_rate_ms,
      },
    }
  }
}
