use colored::{self, Colorize};

#[repr(u8)]
#[derive(Clone, Copy)]
enum SEVERITY {
    FATAL = 2,
    WARN = 4,
    INFO = 5,
    VERBOSE = 6,
    DEBUG = 7,
}

pub struct Logger {
    log_level: u8,
}

impl Logger {
    pub fn new(log_level: u8) -> Self {
        Logger { log_level }
    }

    pub fn verbose(&self, message: &str) {
        if self.log_level >= SEVERITY::VERBOSE as u8 {
            let log_message = format!("[verbose] {message}");
            println!("{}", log_message.truecolor(128, 128, 128))
        }
    }

    pub fn fatal(&self, message: &str) {
        if self.log_level >= SEVERITY::FATAL as u8 {
            let log_message = format!("[fatal] {message}");
            println!("{}", log_message.red())
        }
    }

    pub fn debug(&self, message: &str) {
        if self.log_level >= SEVERITY::DEBUG as u8 {
            let log_message = format!("[debug] {message}");
            println!("{}", log_message.truecolor(128, 128, 128))
        }
    }

    pub fn warn(&self, message: &str) {
        if self.log_level >= SEVERITY::WARN as u8 {
            let log_message = format!("[warn] {message}");
            println!("{}", log_message.yellow())
        }
    }

    pub fn info(&self, message: &str) {
        if self.log_level >= SEVERITY::INFO as u8 {
            let log_message = format!("[info] {message}");
            println!("{}", log_message.green())
        }
    }
}
