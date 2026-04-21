use influxdb::WriteQuery;

use crate::model::MsgProcessingError;
use crate::model::TemperatureReading;

// The crate version of this is try_into but because of our
// strict typing we want a guaranteed success version to simplify things
pub(crate) trait InfluxDbWriteableSafe {
  fn to_query<I: Into<String>>(&self, name: I) -> WriteQuery;
}

impl InfluxDbWriteableSafe for TemperatureReading {
  fn to_query<I: Into<String>>(&self, name: I) -> WriteQuery {
    WriteQuery::new(
      influxdb::Timestamp::Microseconds(self.ts_micro),
      name.into(),
    )
    .add_tag("sensor_id", self.sensor_id.get())
    .add_field("value", self.value)
  }
}

impl InfluxDbWriteableSafe for MsgProcessingError {
  fn to_query<I: Into<String>>(&self, name: I) -> WriteQuery {
    match self {
      MsgProcessingError::MalformedSensorPayload {
        error_code,
        raw_value,
        sensor_id,
        ts_micro,
        is_ingestion_time,
        message,
      } => WriteQuery::new(influxdb::Timestamp::Microseconds(*ts_micro), name.into())
        .add_tag("sensor_id", sensor_id.clone().map(|x| x.get()))
        .add_field("raw_value", raw_value.clone())
        .add_field("error_code", error_code.clone())
        .add_field("message", message.clone())
        .add_field("is_ingestion_time", is_ingestion_time),
    }
  }
}
