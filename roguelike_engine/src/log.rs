use std::fmt;
use std::fs::File;
use std::io::{stdout, Write};

use serde::{Serialize, Deserialize};

use crate::actions::*;

pub struct Log {
    pub action_log: File,
    pub message_log: File,
}

impl Log {
    pub const ACTION_LOG_NAME: &'static str = "action_log.txt";
    pub const MESSAGE_LOG_NAME: &'static str = "message_log.txt";

    pub fn new() -> Log {
        let action_log = File::create(Log::ACTION_LOG_NAME)
                              .expect("Could not open action_log.txt");
        let message_log = File::create(Log::MESSAGE_LOG_NAME)
                               .expect("Could not open messages.txt");

        let log = Log {
            action_log,
            message_log,
        };

        return log;
    }

    pub fn log_action(&mut self, input_action: InputAction) {
        if input_action != InputAction::None {
            self.action_log.write(input_action.to_string().as_bytes()).unwrap();
            self.action_log.write("\n".as_bytes()).unwrap();
        }
    }

    pub fn log_output(&mut self, log_message: &str) {
        self.log(LogMsgType::Output, log_message);
    }

    pub fn log_console(&mut self, log_message: &str) {
        self.log(LogMsgType::Console, log_message);
    }

    pub fn log_msg(&mut self, log_message: &str) {
        self.log(LogMsgType::Msg, log_message);
    }

    pub fn log_key(&mut self, log_message: &str) {
        self.log(LogMsgType::Key, log_message);
    }

    pub fn log_map(&mut self, log_message: &str) {
        self.log(LogMsgType::Map, log_message);
    }

    pub fn log(&mut self, typ: LogMsgType, log_message: &str) {
        let log_msg = format!("{}: {}\n", typ, log_message);
        self.message_log.write(log_msg.as_bytes()).unwrap();
        print!("{}", log_msg);
        stdout().flush().unwrap();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogMsgType {
    Output,
    Console,
    Msg,
    Key,
    Map,
}

impl fmt::Display for LogMsgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogMsgType::Output => write!(f, "OUTPUT"),
            LogMsgType::Console => write!(f, "CONSOLE"),
            LogMsgType::Msg => write!(f, "MSG"),
            LogMsgType::Key => write!(f, "KEY"),
            LogMsgType::Map => write!(f, "MAP"),
        }
    }
}

