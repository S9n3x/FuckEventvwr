use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct ScheduledTaskDetail {
    pub include: bool,
    pub provider: String,
    pub time: String,
    pub event_id: u16,
    pub description: &'static str,
    pub task_name: String,
    pub subject_user_name: String,
    pub action: String,
    pub result_code: String,
    pub raw_data: String,
}

pub struct ScheduledTaskMeta {
    pub time: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub task_name: FieldMeta,
    pub subject_user_name: FieldMeta,
    pub action: FieldMeta,
    pub result_code: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static SCHEDULED_TASK_META: ScheduledTaskMeta = ScheduledTaskMeta {
    time: FieldMeta { title: "时间" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    task_name: FieldMeta {
        title: "任务名称"
    },
    subject_user_name: FieldMeta { title: "操作者" },
    action: FieldMeta {
        title: "执行动作"
    },
    result_code: FieldMeta { title: "结果码" },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
