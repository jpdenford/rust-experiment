use async_stream::stream;
use clap::Parser;
use futures_util::StreamExt;
// use futures_util::stream::stream::StreamExt;
use influxdb::{Client, InfluxDbWriteable, ReadQuery, WriteQuery};
use noise::{NoiseFn, Perlin};
use serde_json::Value;
use std::num::ParseIntError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::select;
use tokio::time;
use tokio::time::sleep;
use tokio_stream::Stream;

use crate::config::Args;
use crate::config::SamplingMode;
mod config;
use crate::exp_utils::now_millis;
mod exp_utils;

type SensorId = u16;

trait ExpMeasurement {
  #[doc = "The time in microseconds that the measurement was taken"]
  fn timestamp_micro(&self) -> u64;
  #[doc = "The global numeric identifier for the sensor"]
  fn sensor_id(&self) -> u16;
}

#[derive(Debug)]
struct TemperatureReading {
  sensor_id: SensorId,
  value: f64,
  ts_micro: u64,
}

impl ExpMeasurement for TemperatureReading {
  fn sensor_id(&self) -> u16 {
    self.sensor_id
  }
  fn timestamp_micro(&self) -> u64 {
    self.ts_micro
  }
}

trait InfluxDbWriteableSafe {
  fn to_query<I: Into<String>>(&self, name: I) -> WriteQuery;
}

impl InfluxDbWriteableSafe for TemperatureReading {
  fn to_query<I: Into<String>>(&self, name: I) -> WriteQuery {
    WriteQuery::new(
      influxdb::Timestamp::Microseconds(self.timestamp_micro() as u128),
      name.into(),
    )
    .add_tag("sensor_id", self.sensor_id())
    .add_field("value", self.value)
  }
}

// Ideally here I'd just use FastCheck or something!
fn dummy_temperature_readings(
  samples_per_second: u16,
  sensor_ids: Vec<SensorId>,
) -> impl Stream<Item = TemperatureReading> {
  let now = now_millis();

  let noise = Perlin::new(now as u32); // wraparound doesn't matter, just need a seed

  // NOTE: this is not very accurate since emitting the measurement will take time
  let micros_between = 1_000_000 / (samples_per_second as u64);

  const MAX_TEMP: f64 = 50_000_000f64;

  stream! {

   let mut interval = time::interval(Duration::from_micros(micros_between));
   println!("Sample interval {:?}", interval.period());

   let mut i = 0usize;

   // TODO add signal/async select for ending the sampling / cleanup?
    loop {
      for sensor_id in &sensor_ids {
        // NOTE: unsafe conversions but not fussed for dummy data
        let point = [(i + (*sensor_id as usize * 200) % 5_000) as f64 / 5_000f64];
        let reading = (noise.get(point) + 1.0) * 0.5 * MAX_TEMP;

        // Okay to unwrap since UNIX_EPOCH will always be earlier than now
        let time_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();

        let dummy_reading = TemperatureReading {
          sensor_id: *sensor_id,
          value: reading,
          // Here the system time is u128 but we won't hit u64::MAX until...a long time
          ts_micro: time_now.try_into().unwrap()
        };

        yield dummy_reading;
      }

      interval.tick().await;

      i = i.wrapping_add(1); // Illustrative only, practically will never wrap
    }
  }
}

#[tokio::main]
async fn main() {
  let config = Args::parse();

  // Note that a stream may or may not be the best choice to simulate the
  // Live configuration. If live, there is a separate process capturing samples, then
  // we'd likely want to use some kind of buffered mechanism (e.g. channel?)
  let sample_stream = match &config.command {
    SamplingMode::Live { address: _ } => todo!("Live mode: Not implemented yet"),
    SamplingMode::Simulated {
      num_sensors,
      sample_rate,
    } => {
      println!("Sampling mode: Simulated");
      // TODO use cancellation token to gracefully shutdown
      dummy_temperature_readings(*sample_rate, (0..*num_sensors).collect())
    }
  };

  let db_client = Client::new(config.db_host, config.db_database).with_token(config.db_token);

  tokio::pin!(sample_stream);
  // There is a fundamental issue with this pipline that it doesn't try to
  // scale up to meet the source 'sample rate'. If the sample velocity is known ahead
  // of time then that's fine but if not then we could lag severely behind
  let pipeline = sample_stream
    .as_mut()
    .ready_chunks(2_000)
    .map(|batch| {
      let queries: Vec<WriteQuery> = batch.iter().map(|v| v.to_query("readings")).collect();
      let client = db_client.clone();
      async move { (client.query(queries).await, batch) }
    })
    .buffer_unordered(4)
    .for_each(async |res| match res {
      (Err(e), _batch) => todo!("log & send to dlq?"),
      (Ok(_), batch) => println!("Batch completed. len: {}", batch.len()),
    });

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

  let res = get_table_count(db_client, "readings").await;

  match res {
    Ok(count) => println!("Row Count: {}", count),
    Err(err) => println!("Failed to get row count: {:?}", err),
  }
}

#[derive(Debug)]
enum CountError {
  DBErr(influxdb::Error),
  ResponseShapeErr(serde_json::Error),
  NumParseErr(ParseIntError),
}

async fn get_table_count(db_client: Client, table_name: &str) -> Result<u64, CountError> {
  db_client
    .query(ReadQuery::new(format!(
      "SELECT count(*) from {} where sensor_id = '1'",
      table_name
    )))
    .await
    .map_err(CountError::DBErr)
    .and_then(|r| {
      let v: Value = serde_json::from_str(r.as_str()).map_err(CountError::ResponseShapeErr)?;
      let count = &v["results"][0]["series"][0]["values"][0][1];
      Ok(count.to_string())
    })
    .and_then(|count_string| count_string.parse::<u64>().map_err(CountError::NumParseErr))
}
