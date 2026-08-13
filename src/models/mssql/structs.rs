use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct MssqlDetail {
    pub include: bool,
    pub time: String,
    pub provider: String,
    pub event_id: u16,
    pub description: &'static str,
    pub event_type: &'static str,
    pub database_instance: String,
    pub raw_data: String,
}

pub struct MssqlMeta {
    pub time: FieldMeta,
    pub provider: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub event_type: FieldMeta,
    pub database_instance: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static MSSQL_META: MssqlMeta = MssqlMeta {
    time: FieldMeta { title: "时间" },
    provider: FieldMeta { title: "Provider" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    event_type: FieldMeta {
        title: "事件类型"
    },
    database_instance: FieldMeta {
        title: "数据库实例",
    },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
