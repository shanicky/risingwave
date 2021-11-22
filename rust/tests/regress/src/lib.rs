#![allow(dead_code)]
#![warn(clippy::map_flatten)]
#![warn(clippy::doc_markdown)]
#![feature(with_options)]
#![feature(path_file_prefix)]

mod opts;

use clap::Parser;
pub(crate) use opts::*;
mod psql;
pub(crate) use psql::*;
mod env;
pub(crate) use env::*;
mod file;
pub(crate) use file::*;
mod schedule;
pub(crate) use schedule::*;

use log::{error, info};

pub async fn regress_main() {
    let opts = Opts::parse();
    log4rs::init_file(opts.log4rs_config_path(), Default::default()).unwrap();

    match run_schedules(opts).await {
        Ok(_) => {
            info!("Risingwave regress test completed successfully!");
        }
        Err(e) => {
            error!("Risingwave regress test failed: {:?}", e);
        }
    }
}

async fn run_schedules(opts: Opts) -> anyhow::Result<()> {
    let schedule = Schedule::new(opts)?;
    schedule.run().await
}