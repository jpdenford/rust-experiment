use noise::{NoiseFn, Perlin};
use tokio::time;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

use async_stream::stream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// struct Params {
//     // Instead of ingesting from the esp-32 generate dummy data
//     simulate: bool,
// }

// trait Sensor<Reading> {
//   fn sample(t: u64) -> Reading;
// }

// struct SensorDynamic {}

// impl Sensor<Temperature> for SensorDynamic {
//   fn sample(t: u64) -> Temperature {
//     Temperature {
//       sensor_id: 1,
//       value: 12123.0,
//     }
//   }
// }

type SensorId = u16;

#[derive(Debug)]
struct TemperatureReading {
  sensor_id: SensorId,
  value: f64,
}

fn dummy_temperature_readings(
  samples_per_second: u16,
  sensor_ids: Vec<SensorId>,
) -> impl Stream<Item = TemperatureReading> {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis();

  let noise = Perlin::new(now as u32); // wraparound doesn't matter, just need a seed

  // NOTE: this is not very accurate since emitting the measurement will take time
  let micros_between = 1_000_000 / (samples_per_second as u64);

  const MAX_TEMP: f64 = 50_000_000f64;

  stream! {

   let mut interval = time::interval(Duration::from_micros(micros_between));
   let mut i = 0usize;

    loop {
      for sensor_id in &sensor_ids {
        // NOTE: unsafe conversions but not fussed for dummy data
        let point = [((i as f64) / 20_000.0) + (*sensor_id as f64 * 20.0)];
        let reading = noise.get(point) * MAX_TEMP;
        let dummy_reading = TemperatureReading {
          sensor_id: *sensor_id,
          value: reading
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
  // TODO use cancellation token to gracefully shutdown
  let readings = dummy_temperature_readings(10, vec![1, 2, 3, 4]);

  tokio::pin!(readings);
  while let Some(v) = readings.next().await {
    println!("Reading: {:?}", v);
  }
}
