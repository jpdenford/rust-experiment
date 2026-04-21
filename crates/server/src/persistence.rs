use crate::model::MsgProcessingError;
use crate::model::TemperatureReading;
use influxdb::Client;
use influxdb::ReadQuery;
use influxdb::WriteQuery;
use std::num::ParseIntError;

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

#[derive(Debug)]
pub enum CountError {
  Db(influxdb::Error),
  ResponseFormat(serde_json::Error),
  NumParse(ParseIntError),
}

pub async fn fetch_measurement_count(
  db_client: Client,
  table_name: &str,
) -> Result<u64, CountError> {
  db_client
    .query(ReadQuery::new(format!(
      "SELECT count(*) from {}",
      table_name
    )))
    .await
    .map_err(CountError::Db)
    .and_then(|r| {
      let v: serde_json::Value =
        serde_json::from_str(r.as_str()).map_err(CountError::ResponseFormat)?;
      let count = &v["results"][0]["series"][0]["values"][0][1];
      Ok(count.to_string())
    })
    .and_then(|count_string| count_string.parse::<u64>().map_err(CountError::NumParse))
}
