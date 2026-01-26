use log::LevelFilter;
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::{
            policy::compound::{
                roll::delete::DeleteRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::fs;
use directories::ProjectDirs;

/// Initializes log4rs with custom configuration for stdout and file logging.
pub fn logger_init() {
    let qual = "in";
    let org = "suyogtandel";
    let app = "picoforge";

    // Determine the log file path using ProjectDirs for cross-platform compatibility
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

    let (app_level, root_level) = if cfg!(debug_assertions) {
        (LevelFilter::Trace, LevelFilter::Debug)
    } else {
        (LevelFilter::Info, LevelFilter::Error)
    };

    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("picoforge", app_level))
        .build(
            Root::builder()
                .appenders(vec!["logfile", "stdout"])
                .build(root_level),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
