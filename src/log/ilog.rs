#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum LogLevel {
    Error = 0,
    Info = 1,
    Warn = 2,
    Debug = 3,
    Verbose = 4,
}

pub struct ILog;

impl ILog {
    const LOG_LEVEL: LogLevel = LogLevel::Debug;
    const ENABLE_LOGGING: bool = true;

    pub fn d(tag: &str, msg: &str) {
        Self::print(LogLevel::Debug, tag, msg);
    }

    #[allow(dead_code)]
    pub fn i(tag: &str, msg: &str) {
        Self::print(LogLevel::Info, tag, msg);
    }

    #[allow(dead_code)]
    pub fn w(tag: &str, msg: &str) {
        Self::print(LogLevel::Warn, tag, msg);
    }

    #[allow(dead_code)]
    pub fn e(tag: &str, msg: &str) {
        Self::print(LogLevel::Error, tag, msg);
    }

    #[allow(dead_code)]
    pub fn v(tag: &str, msg: &str) {
        if !Self::ENABLE_LOGGING {
            return;
        }
        if (Self::LOG_LEVEL as u8) < (LogLevel::Verbose as u8) {
            return;
        }
        Self::print(LogLevel::Verbose, tag, msg);
    }

    fn print(level: LogLevel, tag: &str, msg: &str) {
        if !Self::ENABLE_LOGGING {
            return;
        }

        if (Self::LOG_LEVEL as u8) < (level as u8) {
            return;
        }

        let level = match level {
            LogLevel::Error => "ERR",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Debug => "DEBUG",
            LogLevel::Verbose => "VERBOSE",
        };

        if level == "ERR" {
            eprintln!("{level}::{tag}: {msg}");
        } else {
            println!("{level}::{tag}: {msg}");
        }
    }
}
