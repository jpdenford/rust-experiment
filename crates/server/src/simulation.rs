use crate::model::{MsgProcessingError, PushSensor, SensorId, SensorState, TemperatureReading};
use async_stream::stream;
use futures::Stream;
use std::fmt;
use std::{
  f64::consts::PI,
  sync::Arc,
  time::{Duration, UNIX_EPOCH},
};
use thiserror;
use tokio::time::{interval, sleep};

/// Something that generates raw sample values (no identity or timestamp)
pub trait Generator {
  type Value;
  type Error: fmt::Debug;

  fn generate(&self, t_micros: u128) -> Result<Self::Value, Self::Error>;
}

pub struct SensorSimulated<G: Generator> {
  id: SensorId,
  // Human readable name for debugging (not persisted)
  name: String,
  state: SensorState,
  sample_interval: Duration,
  generator: Arc<G>,
}

impl SensorSimulated<KelvinSineGen> {
  pub fn new(name: &str, id: u16, sample_interval: Duration, generator: KelvinSineGen) -> Self {
    SensorSimulated {
      id: SensorId::new(id),
      name: name.to_string(),
      state: SensorState::Disconnected,
      sample_interval,
      generator: Arc::new(generator),
    }
  }
}

pub struct KelvinSineGen {
  pub min_temp: f64,
  pub max_temp: f64,
  /// Initial phase offset in degrees (course)
  pub phase_offset_rad: f64,
  /// Time to complete a full cycle (in millis)
  pub frequency_ms: u32,
}

impl KelvinSineGen {
  pub fn new() -> Self {
    Self::defaults()
  }

  /// Returns a zeroed/default instance. Use with struct update syntax to override specific fields:
  /// `KelvinSineGen { max_temp: 500.0, ..KelvinSineGen::defaults() }`
  pub fn defaults() -> Self {
    KelvinSineGen {
      min_temp: 0.0,
      max_temp: 100_000_000.0,
      phase_offset_rad: 0.0,
      frequency_ms: 20_000,
    }
  }
}

impl Generator for KelvinSineGen {
  type Value = f64;
  type Error = std::convert::Infallible;

  /// Roughly generates a sin wave (there will certainly be some loss of precision in the calc!)
  fn generate(&self, t_micros: u128) -> Result<Self::Value, Self::Error> {
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

    Ok(value)
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
    self.id.get()
  }

  // Note that this isn't the most efficient since we spawn a task for
  // each sensor. In theory we could combine them into a single thread
  async fn connect_and_sub(
    mut self,
  ) -> Result<impl Stream<Item = Result<Self::Measure, Self::ValueErr>>, Self::ConErr> {
    self.state = SensorState::Connecting;
    // simulate updating the state
    sleep(Duration::from_millis(10)).await;
    self.state = SensorState::Connected;

    let mut interval = interval(self.sample_interval);
    let generator = self.generator.clone();
    let sensor_id = self.id;

    let result_stream = stream! {
      loop {
        interval.tick().await;
        let micros_epoch = UNIX_EPOCH.elapsed().unwrap().as_micros();
        let sample = generator.generate(micros_epoch).map(|value| TemperatureReading {
          sensor_id: sensor_id.clone(),
          ts_micro: micros_epoch,
          value,
        }).map_err(|e| MsgProcessingError::MalformedSensorPayload { // currently no errors are generated
          sensor_id: Some(sensor_id.clone()),
          raw_value: None,
          error_code: "GENERATOR_ERROR".to_string(),
          message: format!("{:?}", e),
          ts_micro: micros_epoch,
          is_ingestion_time: true,
        });
        yield sample;
      }
    };

    Ok(result_stream)
  }

  fn get_state(&self) -> SensorState {
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
      min_temp: 0.0,
      max_temp: 1_000_000.0,
      phase_offset_rad: 0.0,
      frequency_ms: 20_000,
    };

    for t in (0..100_000).step_by(100) {
      let value = generator.generate(t).unwrap();
      assert!(
        value >= generator.min_temp && value <= generator.max_temp,
        "Value out of range: {}",
        value
      );
    }
  }

  #[test]
  #[ignore]
  fn kelvin_sine_phase_offset_accurate() {
    todo!();
  }
}
