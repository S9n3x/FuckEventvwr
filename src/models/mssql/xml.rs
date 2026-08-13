use super::structs::MssqlDetail;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut detail = MssqlDetail::default();
    let mut buffer = Vec::with_capacity(512);
    let mut in_payload = false;
    let mut current_name = String::new();
    let mut data_index = 0usize;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(XmlEvent::Start(ref element)) if element.name().as_ref() == b"Provider" => {
                read_attribute(element, b"Name", &mut detail.provider);
            }
            Ok(XmlEvent::Empty(ref element)) if element.name().as_ref() == b"Provider" => {
                read_attribute(element, b"Name", &mut detail.provider);
            }
            Ok(XmlEvent::Start(ref element)) | Ok(XmlEvent::Empty(ref element))
                if element.name().as_ref() == b"TimeCreated" =>
            {
                if let Some(time) = system_time(element) {
                    detail.time = time;
                }
            }
            Ok(XmlEvent::Start(ref element)) if element.name().as_ref() == b"EventID" => {
                if let Ok(value) = reader.read_text(element.name()) {
                    detail.event_id = value.parse().unwrap_or_default();
                }
            }
            Ok(XmlEvent::Start(ref element))
                if matches!(element.name().as_ref(), b"EventData" | b"UserData") =>
            {
                in_payload = true;
            }
            Ok(XmlEvent::Start(ref element)) if in_payload => {
                current_name.clear();
                if element.name().as_ref() == b"Data" {
                    data_index += 1;
                    for attribute in element.attributes().flatten() {
                        if attribute.key.as_ref() == b"Name" {
                            current_name =
                                String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                            break;
                        }
                    }
                    if current_name.is_empty() {
                        current_name = format!("Data{data_index}");
                    }
                } else {
                    current_name = local_name(element.name().as_ref()).to_owned();
                }
            }
            Ok(XmlEvent::Text(ref text)) if in_payload => {
                if let Ok(value) = text.decode() {
                    let value = value.trim();
                    if !value.is_empty() && value != "-" {
                        assign_audit_result(&mut detail, &current_name, value);
                        append_raw(&mut detail.raw_data, &current_name, value);
                    }
                }
            }
            Ok(XmlEvent::End(ref element)) => match element.name().as_ref() {
                b"EventData" | b"UserData" => {
                    in_payload = false;
                    current_name.clear();
                }
                b"Data" => current_name.clear(),
                _ if in_payload => current_name.clear(),
                _ => {}
            },
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }

    let (event_type, description) = classify(&detail.provider, detail.event_id);
    detail.include = is_mssql_provider(&detail.provider) && is_security_event(detail.event_id);
    detail.database_instance = instance_name(&detail.provider);
    detail.event_type = event_type;
    if detail.result.is_empty() {
        detail.result = default_result(detail.event_id).to_owned();
    }
    detail.description = description;
    if detail.raw_data.ends_with('\n') {
        detail.raw_data.pop();
    }
    Box::new(detail)
}

fn default_result(id: u16) -> &'static str {
    match id {
        18454 => "成功",
        18452 | 18456 | 18470 => "失败",
        17806 | 17832 | 17836 => "异常",
        15281 => "已阻止",
        15457 => "已变更",
        33205 => "",
        _ => "",
    }
}

fn assign_audit_result(detail: &mut MssqlDetail, name: &str, value: &str) {
    if detail.event_id == 33205 && name.eq_ignore_ascii_case("succeeded") {
        match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => detail.result = "成功".to_owned(),
            "false" | "0" => detail.result = "失败".to_owned(),
            _ => {}
        }
    }
}

fn classify(provider: &str, id: u16) -> (&'static str, &'static str) {
    if !is_mssql_provider(provider) {
        return ("未知", "非 MSSQL Provider");
    }
    match id {
        18454 => ("认证成功", "MSSQL: SQL Server 身份验证登录成功"),
        18452 => ("认证失败", "MSSQL: Windows 身份验证来源域不受信任"),
        18456 => ("认证失败", "MSSQL: 登录失败"),
        18470 => ("认证失败", "MSSQL: 已禁用账户登录失败"),
        17806 => ("认证异常", "MSSQL: SSPI 身份验证握手失败"),
        17832 => ("协议异常", "MSSQL: 登录数据包结构无效"),
        17836 => ("协议异常", "MSSQL: 网络数据包长度无效"),
        15281 => ("危险功能", "MSSQL: 尝试访问被安全配置禁用的功能"),
        15457 => ("配置变更", "MSSQL: 服务器配置项已更改"),
        33205 => ("审计事件", "MSSQL: SQL Server Audit 审计事件"),
        _ => ("未知", "未定义 MSSQL 事件"),
    }
}

fn is_security_event(id: u16) -> bool {
    matches!(
        id,
        15281 | 15457 | 17806 | 17832 | 17836 | 18452 | 18454 | 18456 | 18470 | 33205
    )
}

fn is_mssql_provider(provider: &str) -> bool {
    provider.eq_ignore_ascii_case("MSSQLSERVER")
        || provider
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("MSSQL$"))
}

fn instance_name(provider: &str) -> String {
    if provider.eq_ignore_ascii_case("MSSQLSERVER") {
        "MSSQLSERVER".to_owned()
    } else {
        provider
            .split_once('$')
            .map(|(_, value)| value.to_owned())
            .unwrap_or_else(|| provider.to_owned())
    }
}

fn read_attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8], target: &mut String) {
    for attribute in element.attributes().flatten() {
        if attribute.key.as_ref() == name {
            if let Ok(value) = attribute.unescape_value() {
                *target = value.into_owned();
            }
            break;
        }
    }
}

fn local_name(name: &[u8]) -> &str {
    let name = std::str::from_utf8(name).unwrap_or_default();
    name.rsplit(':').next().unwrap_or(name)
}

fn append_raw(raw: &mut String, name: &str, value: &str) {
    raw.push_str(name);
    raw.push_str(": ");
    raw.push_str(value);
    raw.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::definition::CellValue;

    #[test]
    fn emits_only_stable_fields_and_keeps_payload_in_details() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQL$SQLEXPRESS"/><EventID>18456</EventID><TimeCreated SystemTime="2026-08-13T10:00:00Z"/></System><EventData><Data Name="LoginName">sa</Data><Data Name="ClientAddress">10.0.0.8</Data><Data Name="FailureReason">密码不匹配</Data><Data Name="State">8</Data></EventData></Event>"#,
        );
        assert!(event.include());
        let values = event.fields();
        assert!(values.iter().any(|(name, value)| *name == "数据库实例"
            && matches!(value, CellValue::Text(value) if value == "SQLEXPRESS")));
        assert_text_field(&values, "结果", "失败");
        assert_eq!(
            values.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            [
                "时间",
                "Provider",
                "事件ID",
                "描述",
                "事件类型",
                "结果",
                "数据库实例",
                "详细信息"
            ]
        );
        assert!(values.iter().any(|(name, value)| *name == "详细信息" && matches!(value, CellValue::Text(value) if value.contains("LoginName: sa") && value.contains("State: 8"))));
    }

    #[test]
    fn keeps_localized_message_in_details_without_matching_its_text() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQLSERVER"/><EventID>18456</EventID></System><EventData><Data>Login failed for user 'sa'. Reason: Password did not match that for the login provided. [CLIENT: 192.168.1.8]</Data><Data>18456</Data><Data>14</Data><Data>8</Data></EventData></Event>"#,
        );
        let values = event.fields();

        assert!(values.iter().any(|(name, value)| *name == "详细信息"
            && matches!(value, CellValue::Text(value) if value.contains("Login failed for user 'sa'") && value.contains("[CLIENT: 192.168.1.8]"))));
    }

    #[test]
    fn recognizes_audit_result_and_keeps_other_data_in_details() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQLSERVER"/><EventID>33205</EventID></System><EventData><Data Name="database_principal_name">dbo</Data><Data Name="server_principal_name">audit_context</Data><Data Name="session_server_principal_name">DOMAIN\analyst</Data><Data Name="client_ip">10.0.0.20</Data><Data Name="succeeded">false</Data></EventData></Event>"#,
        );
        let values = event.fields();

        assert_text_field(&values, "结果", "失败");
        assert!(values.iter().any(|(name, value)| *name == "详细信息"
            && matches!(value, CellValue::Text(value) if value.contains("server_principal_name: audit_context") && value.contains("client_ip: 10.0.0.20"))));
    }

    #[test]
    fn rejects_same_event_id_from_other_provider() {
        assert!(!parse(r#"<Event><System><Provider Name="Other"/><EventID>18456</EventID></System></Event>"#).include());
    }

    fn assert_text_field(
        values: &[(&'static str, CellValue<'_>)],
        expected_name: &str,
        expected_value: &str,
    ) {
        assert!(values.iter().any(|(name, value)| {
            *name == expected_name
                && matches!(value, CellValue::Text(value) if value == expected_value)
        }));
    }
}
