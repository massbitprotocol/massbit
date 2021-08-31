/**
 *** This file is to help setup the logger based on the RUST_LOG and RUST_LOG_TYPE options
 **/
use chrono::Local;
use env_logger::Builder;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::io::Write;

pub fn log_to_file(file_name: &String, log_level: &String) {
    let date = Local::now().format("%Y-%m-%dT%H:%M:%S");
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::default()))
        .build(format!("log/{}-{}.log", file_name, date)) // set the file name based on the current date
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(log_level.parse().unwrap()),
        )
        .unwrap();
    log4rs::init_config(config).unwrap();
}

pub fn log_to_console(log_level: &String) {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - [{}] {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"), // Reformat to human-readable timestamp
                record.level(),
                record.module_path().unwrap(),
                record.args(),
            )
        })
        .filter(None, log_level.parse().unwrap())
        .init();
}

pub fn message(output_type: &String, level: &String) -> String {
    format!(
        "Logger will now output to {} with the level: {}",
        output_type, level
    )
}
