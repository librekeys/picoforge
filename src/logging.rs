use directories::ProjectDirs;
use log::LevelFilter;
use log::Record;
use log4rs::{
    append::{
        Append,
        console::{ConsoleAppender, Target},
        rolling_file::{
            RollingFileAppender,
            policy::compound::{
                CompoundPolicy, roll::delete::DeleteRoller, trigger::size::SizeTrigger,
            },
        },
    },
    config::{Appender, Logger, Root},
    encode::{Encode, Write, pattern::PatternEncoder},
};
use std::fs;
use std::sync::{Arc, Mutex, OnceLock};

pub static LOG_BUFFER: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();

struct SimpleWriter(Vec<u8>);

impl std::io::Write for SimpleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for SimpleWriter {}

#[derive(Debug)]
struct BufferAppender {
    encoder: Box<dyn Encode>,
}

impl BufferAppender {
    pub fn new(encoder: Box<dyn Encode>) -> Self {
        Self { encoder }
    }
}

impl Append for BufferAppender {
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let mut writer = SimpleWriter(Vec::new());
        self.encoder.encode(&mut writer, record)?;
        let log = String::from_utf8(writer.0)?;

        let buffer = LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
        if let Ok(mut logs) = buffer.lock() {
            if logs.len() >= 1000 {
                logs.remove(0);
            }
            logs.push(log);
        }
        Ok(())
    }

    fn flush(&self) {}
}

/// Initializes log4rs with custom configuration for stdout and file logging.
pub fn logger_init() {
    let qual = "in";
    let org = "suyogtandel";
    let app = "picoforge";

    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(Vec::new())));

    let log_file_path = {
        let log_dir = if let Some(proj_dirs) = ProjectDirs::from(qual, org, app) {
            proj_dirs.data_local_dir().join("logs")
        } else {
            eprintln!("Could not determine project directories. Falling back to local directory.");
            std::path::PathBuf::from("logs")
        };

        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory at {:?}: {}", log_dir, e);
        }

        log_dir.join("picoforge.log")
    };

    // TODO: Add session based log files or rolling log files with archiving of old files, to prevent a single log file from growing too large.
    let size_trigger = SizeTrigger::new(10 * 1024 * 1024); // 10 MB limit
    let roller = DeleteRoller::new();
    let policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));

    // File Appender
    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t}] {m}{n}",
        )))
        .build(log_file_path, Box::new(policy))
        .unwrap();

    // Console Appender
    let stdout = ConsoleAppender::builder()
        .target(Target::Stdout)
        .encoder(Box::new(PatternEncoder::new(
            "[{d(%Y-%m-%d %H:%M:%S %Z)} {h({l})} {t}] {m}{n}",
        )))
        .build();

    // Buffer Appender
    let buffer_appender = BufferAppender::new(Box::new(PatternEncoder::new(
        "[{d(%Y-%m-%d %H:%M:%S %Z)} {l} {t}] {m}{n}",
    )));

    let app_level = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .appender(Appender::builder().build("buffer", Box::new(buffer_appender) as Box<dyn Append>))
        .logger(
            Logger::builder()
                .appenders(["stdout", "logfile", "buffer"])
                .additive(false)
                .build("picoforge", app_level),
        )
        .logger(Logger::builder().build("gpui", LevelFilter::Error))
        .logger(Logger::builder().build("gpui_component", LevelFilter::Error))
        .logger(Logger::builder().build("blade_graphics", LevelFilter::Error))
        .build(
            Root::builder()
                .appenders(vec!["logfile", "stdout", "buffer"])
                .build(LevelFilter::Error),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
