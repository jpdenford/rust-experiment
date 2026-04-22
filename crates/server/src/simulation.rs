use crate::model::{MsgProcessingError, PushSensor, SensorId, State, TemperatureReading};
use async_stream::stream;
use futures::Stream;
use std::{
  f64::consts::PI,
  sync::Arc,
  time::{Duration, UNIX_EPOCH},
};
use thiserror;
use tokio::time::{interval, sleep};

/// Something that generates sensor readings and failures
pub trait Generator {
  type Measurement;
  type FailedMeasurement;

  fn generate(&self, t_micros: u128) -> Result<Self::Measurement, Self::FailedMeasurement>;
}

pub struct SensorSimulated<G: Generator> {
  id: u16,
  name: String,
  state: State,
  sample_interval: Duration,
  generator: Arc<G>,
}

impl SensorSimulated<KelvinSineGen> {
  pub fn new(name: &str, id: u16, sample_interval: Duration, generator: KelvinSineGen) -> Self {
    SensorSimulated {
      id,
      name: name.to_string(),
      state: State::Disconnected,
      sample_interval,
      generator: Arc::new(generator),
    }
  }
}

pub struct KelvinSineGen {
  pub sensor_id: SensorId,

  pub min_temp: f64,
  pub max_temp: f64,
  /// Initial phase offset in degrees (course)
  pub phase_offset_rad: f64,
  /// Time to complete a full cycle (in millis)
  pub frequency_ms: u32,
}

impl KelvinSineGen {
  pub fn new(sensor_id: SensorId) -> Self {
    KelvinSineGen {
      sensor_id,
      ..Self::defaults()
    }
  }

  /// Returns a zeroed/default instance. Use with struct update syntax to override specific fields:
  /// `KelvinSineGen { sensor_id, max_temp: 500.0, ..KelvinSineGen::defaults() }`
  pub fn defaults() -> Self {
    KelvinSineGen {
      sensor_id: SensorId::new(0),
      min_temp: 0.0,
      max_temp: 100_000_000.0,
      phase_offset_rad: 0.0,
      frequency_ms: 20_000,
    }
  }
}

impl Generator for KelvinSineGen {
  type Measurement = TemperatureReading;
  type FailedMeasurement = MsgProcessingError;

  /// Roughly generates a sin wave (there will certainly be some loss of precision in the calc!)
  fn generate(&self, t_micros: u128) -> Result<Self::Measurement, Self::FailedMeasurement> {
    let two_pi = 2.0 * PI;
    let freq_micros = u64::from(self.frequency_ms) * 1000;
    // we know it fits b'cos freq_micros is a u64!
    let t_mod_freq = (t_micros % u128::from(freq_micros)) as u64;
    // frequency as 0 - 1
    let freq_normalised = t_mod_freq as f64 / freq_micros as f64; // not safe
    let phased = two_pi * (freq_normalised + (f64::from(self.phase_offset_rad) / 365.0));

    let sin = phased.sin();
    let amplitude_normalised = (sin + 1.0) / 2.0;

    // now shift the sin to our specified domain / range
    let temp_range = self.max_temp - self.min_temp;
    let value = (amplitude_normalised * temp_range) + self.min_temp;

    Ok(TemperatureReading {
      sensor_id: self.sensor_id.clone(),
      ts_micro: t_micros,
      value,
    })
  }
}

#[derive(thiserror::Error, Debug, Clone)]
#[error("Cannot connect: {reason}")]
pub struct CannotConnect {
  reason: String,
}

// TODO try using a 'blanket implementation'
// https://www.youtube.com/watch?v=qrf52BVaZM8
// e.g. impl<I: Iterator> IteratorExt for I {}

impl PushSensor for SensorSimulated<KelvinSineGen> {
  type ConErr = CannotConnect;
  type Measure = TemperatureReading;
  type ValueErr = MsgProcessingError;

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

#[cfg(test)]
mod tests {

  use super::*;

  // TODO use fastcheck to explore the generated space more thoroughly
  #[test]
  fn kelvin_sine_gen_within_range() {
    let generator = KelvinSineGen {
      sensor_id: SensorId::new(0),
      min_temp: 0.0,
      max_temp: 1_000_000.0,
      phase_offset_rad: 0.0,
      frequency_ms: 20_000,
    };

    for t in (0..100_000).step_by(100) {
      let reading = generator.generate(t).unwrap();
      assert!(
        reading.value >= generator.min_temp && reading.value <= generator.max_temp,
        "Value out of range: {}",
        reading.value
      );
    }
  }

  #[test]
  #[ignore]
  fn kelvin_sine_phase_offset_accurate() {
    todo!();
  }
}
