use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
pub struct Args {
  #[command(subcommand)]
  pub command: SamplingMode,

  #[arg(long, env = "DURATION")]
  pub duration: u16,

  #[arg(long, env = "DB_HOST")]
  pub db_host: String,

  #[arg(long, env = "DB_DATABASE")]
  pub db_database: String,

  #[arg(long, env = "DB_TOKEN")]
  pub db_token: String,
}

#[derive(Subcommand, Debug, Clone)]
pub enum SamplingMode {
  Simulated {
    #[doc = "The number of sensors to simulate"]
    #[arg(long, short = 'n', default_value_t = 1u16)]
    num_sensors: u16,

    #[doc = "The number of samples per sensor per second"]
    #[arg(long, short = 's', default_value_t = 10u16)]
    sample_rate: u16,
  },
  Live {
    #[doc = "The addresses of any real sensors"] // TBD how to actually do this w. esp32 etc
    #[arg(long)]
    address: Vec<String>,
  },
}
