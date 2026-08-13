use clap::{Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub model: Models,
    pub format: OutFormat,
    pub jobs: usize,
}

pub struct ModelRule {
    pub files: Vec<&'static str>,
    pub ids: Vec<u16>,
}

#[derive(Subcommand, Debug, Clone)]
#[command(rename_all = "verbatim")]
pub enum Models {
    /// 程序活动, EventID: 1000, 1001, 1002, 1026, 1033, 1034, 1040, 1042, 11707, 11708, 11724
    Application,
    /// 验证相关信息, EventID: 4624, 4625, 4648, 4672, 4740, 4768, 4771, 4776，4769
    Authentication,
    /// 会话生命周期, EventID: 4634, 4647, 21, 22, 23, 24, 25, 40, 1149
    Session,
    /// 账户管理, EventID: 4720, 4722, 4723, 4724, 4725, 4726, 4738, 4740, 4767
    AccountManagement,
    /// 服务管理, EventID: 7034, 7035, 7036, 7040, 7045
    ServiceControl,
    /// 计划任务, EventID: 4698, 4699, 4700, 4701, 4702, 106, 140, 141, 200, 201
    ScheduledTask,
    /// PowerShell 执行日志, EventID: 4103, 4104, 4105, 4106, 400, 403, 600
    PowerShell,
}

impl Models {
    pub const fn command_name(&self) -> &'static str {
        match self {
            Self::Application => "Application",
            Self::Authentication => "Authentication",
            Self::Session => "Session",
            Self::AccountManagement => "AccountManagement",
            Self::ServiceControl => "ServiceControl",
            Self::ScheduledTask => "ScheduledTask",
            Self::PowerShell => "PowerShell",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutFormat {
    Csv,
    Excel,
}
