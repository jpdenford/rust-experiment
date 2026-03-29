use noise::{Clamp, NoiseFn, Perlin};
use tokio_stream::Stream;
use tokio_stream::{self as stream, StreamExt};

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
struct Temperature {
  sensor_id: SensorId,
  value: f64,
}

fn dummy_values(
  _evts_per_second: u16,
  sensor_ids: Vec<SensorId>,
) -> impl Stream<Item = Temperature> {
  // TODO: spin up new thread and pump messages into the stream
  let noise = Perlin::new(1); // new seed each time?
  let samples = (1..200).map(move |v| {
    let idx = v % sensor_ids.len();
    // TODO make realistic temp readings
    let point = [(v as f64) / 200.0];
    let reading = noise.get(point);
    println!("reading {}", reading);
    Temperature {
      sensor_id: sensor_ids[idx], // okay
      value: reading,
    }
  });
  stream::iter(samples)
}

#[tokio::main]
async fn main() {
  let mut stream = dummy_values(10, vec![1, 2, 3, 4]);
  while let Some(v) = stream.next().await {
    println!("{:?}", v);
  }
  println!("done")
}
