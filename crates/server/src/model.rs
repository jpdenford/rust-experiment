use futures::Stream;

// pub type SensorId = u16;

#[derive(Debug, Clone)]
pub struct SensorId(u16);
impl SensorId {
  pub fn new(id: u16) -> Self {
    SensorId(id)
  }
  pub fn get(&self) -> u16 {
    self.0
  }
}

pub trait ExpMeasurement {
  #[doc = "The time in microseconds that the measurement was taken"]
  fn timestamp_micro(&self) -> u128;
  #[doc = "The global numeric identifier for the sensor"]
  fn sensor_id(&self) -> u16;
}

impl ExpMeasurement for TemperatureReading {
  fn sensor_id(&self) -> u16 {
    self.sensor_id.0
  }

  fn timestamp_micro(&self) -> u128 {
    self.ts_micro
  }
}

#[derive(Debug, Clone)]
pub struct MalformedSensorPayload {
  pub sensor_id: Option<SensorId>,
  /// Hex encoded raw values received over the wire
  pub raw_value: Option<String>,
  /// Canonical error code representing the type
  pub error_code: String, // Alternatively can make this struct into an enum and use a 'getter fn' at storage layer
  /// Detail about why the message couldn't be properly read
  pub message: String,
  /// Either the sensor or ingestion time
  pub ts_micro: u128,
  /// if the payload is malformed we may need to use the
  /// ingestion time rather than the 'sensor time' on the payload
  pub is_ingestion_time: bool,
}

#[derive(Debug, Clone)]
pub struct TemperatureReading {
  pub sensor_id: SensorId,
  pub value: f64,
  pub ts_micro: u128,
}

#[derive(Clone)]
pub enum State {
  Disconnected,
  Connecting,
  Connected,
  Failed { reason: String, retries: String },
}

/// Receives values
pub trait PushSensor {
  type ConErr;
  type ValueErr;
  type Measure;

  fn name(&self) -> &str;
  fn id(&self) -> u16;

  async fn connect_and_sub(
    self,
  ) -> Result<impl Stream<Item = Result<Self::Measure, Self::ValueErr>>, Self::ConErr>;

  fn get_state(&self) -> State;
}
