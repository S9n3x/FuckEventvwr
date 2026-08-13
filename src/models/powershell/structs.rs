use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct PowerShellDetail {
    pub time: String,
    pub event_id: u16,
    pub description: &'static str,
    pub script_block: String,
    pub command_line: String,
    pub raw_data: String,
}

pub struct PowerShellMeta {
    pub time: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub script_block: FieldMeta,
    pub command_line: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static POWER_SHELL_META: PowerShellMeta = PowerShellMeta {
    time: FieldMeta { title: "时间" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    script_block: FieldMeta {
        title: "脚本内容"
    },
    command_line: FieldMeta { title: "命令行" },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
