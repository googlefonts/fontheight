#![allow(missing_docs)]

use anyhow::anyhow;
use pico_args::Arguments;

mod egg;

fn main() -> anyhow::Result<()> {
    let mut args = Arguments::from_env();

    match args.subcommand()?.ok_or(anyhow!("missing task"))?.as_str() {
        "egg" => egg::main(args),
        unknown => Err(anyhow!("unknown task: {unknown}")),
    }
}
