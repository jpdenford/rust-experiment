use crate::persistence::InfluxDbWriteableSafe;
use crate::simulation::{KelvinSineGen, SensorSimulated};
use crate::{
  model::{MsgProcessingError, PushSensor, SensorId},
  simulation::CannotConnect,
};
use backon::{ExponentialBuilder, Retryable};
use futures::{StreamExt, future::try_join_all, stream::select_all};
use influxdb::{Client, WriteQuery};
use itertools::Itertools;
use std::time::Duration;
use tokio::pin;

pub enum IngestionConfig {
  Simulated { num_sensors: u16, sample_rate: u16 },
  Live { addresses: Vec<String> },
}

pub async fn run_ingestion<Cfg: Into<IngestionConfig>>(
  config: Cfg,
  db_client: Client,
) -> Result<(), CannotConnect> {
  let sensors = match &config.into() {
    IngestionConfig::Live { .. } => todo!("Live mode: Not implemented yet"),
    IngestionConfig::Simulated {
      num_sensors,
      sample_rate,
    } => {
      // println!("Sampling mode: Simulated");
      // // TODO use cancellation token to gracefully shutdown
      // dummy_temperature_readings(*sample_rate, (0..*num_sensors).collect())
      (0..*num_sensors).map(|i| {
        SensorSimulated::<KelvinSineGen>::new(
          format!("temp_{}", i).as_str(),
          i,
          Duration::from_millis(1),
          KelvinSineGen::new(SensorId::new(i)),
        )
      })
    }
  };

  // TODO stream / select_all
  let subs = try_join_all(sensors.map(|s| s.connect_and_sub()))
    .await?
    .into_iter()
    .map(Box::pin);

  let subs_combined = select_all(subs);

  pin!(subs_combined);

  // There is a fundamental issue with this pipline that it doesn't try to
  // scale up to meet the source 'sample rate'. If the sample velocity is known ahead
  // of time then that's fine but if not then we could lag severely behind.
  // This is only a problem if the sample rate isn't known ahead of time / can change dynamically though.
  subs_combined
    .ready_chunks(2_000)
    .map(|batch| {
      let (readings, failures): (Vec<_>, Vec<_>) = batch.clone().into_iter().partition_result();

      let valid_readings: Vec<WriteQuery> =
        readings.iter().map(|v| v.to_query("readings")).collect();

      let invalid_readings: Vec<WriteQuery> = failures
        .iter()
        .map(|v| v.to_query("readings_invalid"))
        .collect();

      let all_readings = [valid_readings.as_slice(), invalid_readings.as_slice()].concat();

      let client = db_client.clone();
      async move {
        (
          (|| client.query(all_readings.clone()))
            .retry(ExponentialBuilder::default())
            .when(|e| match *e {
              influxdb::Error::ApiError(_) => true, // TODO add more here - illustrative only
              _ => false,
            })
            .notify(|err, dur: Duration| {
              println!(
                "retrying influx insert, encountered: {:?} after {:?}",
                err, dur
              ); // TODO use propper logger
            })
            .await,
          batch,
        )
      }
    })
    .buffer_unordered(4) // four batches in flight at any given time
    .for_each(async |res| match res {
      (Err(e), _batch) => todo!("log & send to dlq?"),
      (Ok(_), batch) => println!("Batch completed. len: {}", batch.len()),
    })
    .await;
  Ok(())
}

async fn _handle_failed_inserts(_readings: Vec<MsgProcessingError>) -> () {
  // Write failures to dead-letter file on disk? (append only)
  todo!();
}
