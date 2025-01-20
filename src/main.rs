use env_logger::Env;
use log::LevelFilter;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Warn)
        .parse_env(Env::new().filter("FONTHEIGHT_LOG"))
        .init();
    println!("Hello, world!");
}
