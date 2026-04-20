use crate::{
  exp_utils::now_millis,
  model::{MalformedSensorPayload, PushSensor, SensorId, State, TemperatureReading},
};
use async_stream::stream;
use futures::Stream;
use noise::{NoiseFn, Perlin};
use std::{
  f64::consts::PI,
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::{self, interval, sleep};

/// Something that generates sensor readings and failures
pub trait Generator {
  type Output;
  type Error;

  fn generate(&self, t_micros: u128) -> Result<Self::Output, Self::Error>;
}

pub struct SensorSimulated<G: Generator> {
  id: u16,
  name: String,
  state: State,
  sample_interval: Duration,
  generator: Arc<G>,
}

impl SensorSimulated<PerlinKelvinGen> {
  fn new(name: &str, id: u16, sample_interval: Duration, generator: PerlinKelvinGen) -> Self {
    SensorSimulated {
      id,
      name: name.to_string(),
      state: State::Disconnected,
      sample_interval,
      generator: Arc::new(generator),
    }
  }
}

impl SensorSimulated<SineKelvinGen> {
  pub fn new(name: &str, id: u16, sample_interval: Duration, generator: SineKelvinGen) -> Self {
    SensorSimulated {
      id,
      name: name.to_string(),
      state: State::Disconnected,
      sample_interval,
      generator: Arc::new(generator),
    }
  }
}

#[derive(Debug)]
pub struct CannotConnect {
  reason: String,
}

struct PerlinKelvinGen;
impl Generator for PerlinKelvinGen {
  type Output = TemperatureReading;
  type Error = MalformedSensorPayload;

  fn generate(&self, t_micros: u128) -> Result<Self::Output, Self::Error> {
    // TODO
    Ok(TemperatureReading {
      sensor_id: SensorId::new(1),
      value: 1.0,
      ts_micro: 3,
    })
  }
}

// struct Microseconds(u128);

pub struct SineKelvinGen {
  pub sensor_id: SensorId,

  pub min_temp: f64,
  pub max_temp: f64,
  /// Initial phase offset in degrees (course)
  pub phase_offset_deg: u8,
  /// Time to complete a full cycle (in millis)
  pub frequency_ms: u32,
}

impl SineKelvinGen {
  pub fn new(sensor_id: SensorId) -> Self {
    SineKelvinGen {
      sensor_id,
      min_temp: 0.0,
      max_temp: 100_000_000.0,
      phase_offset_deg: 0,
      frequency_ms: 20_000,
    }
  }
}

impl Generator for SineKelvinGen {
  type Output = TemperatureReading;
  type Error = MalformedSensorPayload;

  fn generate(&self, t_micros: u128) -> Result<Self::Output, Self::Error> {
    let two_pi = 2.0 * PI;
    let freq_micros = u64::from(self.frequency_ms) * 1000;
    // we know it fits b'cos freq_micros is a u64!
    let ranged = (t_micros % u128::from(freq_micros)) as u64;
    let normalised = ranged as f64 / freq_micros as f64; // not safe
    let value = two_pi * (normalised + (f64::from(self.phase_offset_deg) / 365.0));

    Ok(TemperatureReading {
      sensor_id: self.sensor_id.clone(),
      ts_micro: t_micros,
      value,
    })
  }
}

// TODO try using a 'blanket implementation'
// https://www.youtube.com/watch?v=qrf52BVaZM8
// e.g. impl<I: Iterator> IteratorExt for I {}

impl PushSensor for SensorSimulated<SineKelvinGen> {
  type ConErr = CannotConnect;
  type Measure = TemperatureReading;
  type ValueErr = MalformedSensorPayload;

  fn name(&self) -> &str {
    &self.name
  }

  fn id(&self) -> u16 {
    self.id
  }

  // Note that this isn't the most efficient since we spawn a task for
  // each sensor. In theory we could combine them intongle thread.
  async fn connect_and_sub(
    mut self,
  ) -> Result<impl Stream<Item = Result<Self::Measure, Self::ValueErr>>, Self::ConErr> {
    self.state = State::Connecting;
    // simulate updating the state
    sleep(Duration::from_millis(10)).await;
    self.state = State::Connected;

    let mut interval = interval(self.sample_interval);
    let generator = self.generator.clone();

    let result_stream = stream! {
      loop {
        interval.tick().await;
        let t = UNIX_EPOCH.elapsed().unwrap().as_micros();
        let sample = generator.generate(t);
        yield sample;
      }
    };

    Ok(result_stream)
  }

  fn get_state(&self) -> State {
    self.state.clone()
  }
}

// Ideally here I'd just use FastCheck or something!
fn _dummy_temperature_readings(
  samples_per_second: u16,
  sensor_ids: Vec<SensorId>,
) -> impl Stream<Item = TemperatureReading> {
  let now = now_millis();

  let noise = Perlin::new((now % u128::from(u32::MAX)) as u32); // wraparound doesn't matter, just need a seed

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
        let point = [(i + ((*sensor_id).get() as usize * 200) % 5_000) as f64 / 5_000f64];
        let reading = (noise.get(point) + 1.0) * 0.5 * MAX_TEMP;

        // Okay to unwrap since UNIX_EPOCH will always be earlier than now
        let time_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();

        let dummy_reading = TemperatureReading {
          sensor_id: (*sensor_id).clone(),
          value: reading,
          ts_micro: time_now
        };

        yield dummy_reading;
      }

      interval.tick().await;

      i = i.wrapping_add(1); // Illustrative only, practically will never wrap
    }
  }
}
