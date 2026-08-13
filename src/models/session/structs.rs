use crate::parser::definition::FieldMeta;

#[derive(Debug, Default)]
pub struct SessionDetail {
    pub include: bool,
    pub time: String,
    pub provider: String,
    pub event_id: u16,
    pub description: &'static str,
    pub user_name: String,
    pub domain_name: String,
    pub session_id: String,
    pub logon_type: Option<u32>,
    pub remote_address: String,
    pub reason: String,
    pub raw_data: String,
}

pub struct SessionMeta {
    pub time: FieldMeta,
    pub provider: FieldMeta,
    pub event_id: FieldMeta,
    pub description: FieldMeta,
    pub user_name: FieldMeta,
    pub domain_name: FieldMeta,
    pub session_id: FieldMeta,
    pub logon_type: FieldMeta,
    pub remote_address: FieldMeta,
    pub reason: FieldMeta,
    pub raw_data: FieldMeta,
}

pub static SESSION_META: SessionMeta = SessionMeta {
    time: FieldMeta { title: "时间" },
    provider: FieldMeta { title: "Provider" },
    event_id: FieldMeta { title: "事件ID" },
    description: FieldMeta { title: "描述" },
    user_name: FieldMeta { title: "用户" },
    domain_name: FieldMeta { title: "域" },
    session_id: FieldMeta { title: "会话ID" },
    logon_type: FieldMeta {
        title: "登录类型"
    },
    remote_address: FieldMeta {
        title: "来源地址"
    },
    reason: FieldMeta { title: "原因码" },
    raw_data: FieldMeta {
        title: "详细信息"
    },
};
