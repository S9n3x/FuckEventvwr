use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct ApplicationDetail {
    pub include: bool,
    pub time: String,
    pub provider: String,
    pub event_id: u16,
    pub description: &'static str,
    pub event_type: String,
    pub report_type: String,
    pub application_name: String,
    pub application_version: String,
    pub application_path: String,
    pub fault_module: String,
    pub fault_module_path: String,
    pub exception_code: String,
    pub process_id: String,
    pub raw_data: String,
}

pub struct ApplicationMeta {
    pub time: FieldMeta,
    pub provider: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub event_type: FieldMeta,
    pub report_type: FieldMeta,
    pub application_name: FieldMeta,
    pub application_version: FieldMeta,
    pub application_path: FieldMeta,
    pub fault_module: FieldMeta,
    pub fault_module_path: FieldMeta,
    pub exception_code: FieldMeta,
    pub process_id: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static APPLICATION_META: ApplicationMeta = ApplicationMeta {
    time: FieldMeta { title: "时间" },
    provider: FieldMeta { title: "Provider" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    event_type: FieldMeta {
        title: "事件类型"
    },
    report_type: FieldMeta {
        title: "报告类型"
    },
    application_name: FieldMeta {
        title: "程序名称"
    },
    application_version: FieldMeta {
        title: "程序版本"
    },
    application_path: FieldMeta {
        title: "程序路径"
    },
    fault_module: FieldMeta {
        title: "故障模块"
    },
    fault_module_path: FieldMeta {
        title: "故障模块路径",
    },
    exception_code: FieldMeta {
        title: "异常代码"
    },
    process_id: FieldMeta { title: "进程ID" },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
