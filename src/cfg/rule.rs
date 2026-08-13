use super::sturct::*;

impl Models {
    pub fn get_model_rule(&self) -> ModelRule {
        match self {
            Models::Application => ModelRule {
                files: vec!["Application.evtx"],
                ids: vec![
                    1000, 1001, 1002, 1026, 1033, 1034, 1040, 1042, 11707, 11708, 11724,
                ],
            },

            // 验证相关
            Models::Authentication => ModelRule {
                files: vec!["Security.evtx"],
                ids: vec![4624, 4625, 4648, 4672, 4740, 4768, 4771, 4776, 4769],
            },

            // 会话生命周期
            Models::Session => ModelRule {
                files: vec![
                    "Security.evtx",
                    "Microsoft-Windows-TerminalServices-LocalSessionManager%4Operational.evtx",
                    "Microsoft-Windows-TerminalServices-RemoteConnectionManager%4Operational.evtx",
                ],
                ids: vec![4634, 4647, 21, 22, 23, 24, 25, 40, 1149],
            },

            // 账户管理
            Models::AccountManagement => ModelRule {
                files: vec!["Security.evtx"],
                ids: vec![4720, 4722, 4723, 4724, 4725, 4726, 4738, 4740, 4767],
            },

            // 服务管理
            Models::ServiceControl => ModelRule {
                files: vec!["System.evtx"],
                ids: vec![7034, 7035, 7036, 7040, 7045],
            },

            // 计划任务
            Models::ScheduledTask => ModelRule {
                files: vec![
                    "Security.evtx",
                    "Microsoft-Windows-TaskScheduler%4Operational.evtx",
                ],
                ids: vec![4698, 4699, 4700, 4701, 4702, 106, 140, 141, 200, 201],
            },

            // PowerShell 执行日志
            Models::PowerShell => ModelRule {
                files: vec![
                    "Microsoft-Windows-PowerShell%4Operational.evtx",
                    "Windows PowerShell.evtx",
                ],
                ids: vec![4103, 4104, 4105, 4106, 400, 403, 600],
            },
        }
    }
}
