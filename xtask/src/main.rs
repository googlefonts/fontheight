#![allow(missing_docs)]

use std::error::Error;

use pico_args::Arguments;

mod slwl_build;

type Result<T = (), E = Box<dyn Error>> = std::result::Result<T, E>;

fn main() -> Result {
    let mut args = Arguments::from_env();

    match args.subcommand()?.ok_or("missing task")?.as_str() {
        "slwl-build" => slwl_build::main(args),
        unknown => Err(format!("unknown task: {unknown}").into()),
    }
}
