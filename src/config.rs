use std::env;

pub struct Config {
    pub port: u32,
}

impl Config {
    pub fn new() -> Config {
        let port = env::var("PORT")
            .unwrap_or("3000".to_string())
            .parse::<u32>()
            .unwrap();

        Config { port }
    }
}
