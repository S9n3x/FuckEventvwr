use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct ServiceControlDetail {
    pub include: bool,
    pub time: String,
    pub provider: String,
    pub event_id: u16,
    pub description: &'static str,
    pub service_name: String,
    pub internal_service_name: String,
    pub old_start_type: String,
    pub new_start_type: String,
    pub service_type: String,
    pub start_type: String,
    pub image_path: String,
    pub raw_data: String,
}

pub struct ServiceControlMeta {
    pub time: FieldMeta,
    pub provider: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub service_name: FieldMeta,
    pub internal_service_name: FieldMeta,
    pub old_start_type: FieldMeta,
    pub new_start_type: FieldMeta,
    pub service_type: FieldMeta,
    pub start_type: FieldMeta,
    pub image_path: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static SERVICE_CONTROL_META: ServiceControlMeta = ServiceControlMeta {
    time: FieldMeta { title: "时间" },
    provider: FieldMeta { title: "Provider" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    service_name: FieldMeta {
        title: "服务名称"
    },
    internal_service_name: FieldMeta {
        title: "内部服务名",
    },
    old_start_type: FieldMeta {
        title: "旧启动类型",
    },
    new_start_type: FieldMeta {
        title: "新启动类型",
    },
    service_type: FieldMeta {
        title: "服务类型"
    },
    start_type: FieldMeta {
        title: "启动类型"
    },
    image_path: FieldMeta {
        title: "可执行路径",
    },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
