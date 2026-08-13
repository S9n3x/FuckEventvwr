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
                        assign_common_field(&mut detail, &current_name, value);
                        extract_message_fields(&mut detail, value);
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
        33205 => "审计记录",
        _ => "",
    }
}

fn assign_common_field(detail: &mut MssqlDetail, name: &str, value: &str) {
    if matches_name(
        name,
        &[
            "LoginName",
            "UserName",
            "User",
            "ServerPrincipalName",
            "SessionServerPrincipalName",
            "DatabasePrincipalName",
            "PrincipalName",
            "server_principal_name",
            "session_server_principal_name",
            "database_principal_name",
        ],
    ) {
        copy_if_empty(&mut detail.user, value);
    } else if matches_name(
        name,
        &[
            "Client",
            "ClientIP",
            "ClientAddress",
            "IpAddress",
            "IPAddress",
            "RemoteAddress",
            "client_ip",
        ],
    ) {
        copy_if_empty(&mut detail.client_address, value);
    } else if matches_name(
        name,
        &["Reason", "FailureReason", "ErrorMessage", "failure_reason"],
    ) {
        copy_if_empty(&mut detail.failure_reason, value);
    } else if detail.event_id == 33205
        && matches_name(name, &["Succeeded", "succeeded", "IsSuccess"])
    {
        detail.result = match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => "成功".to_owned(),
            "false" | "0" | "no" => "失败".to_owned(),
            _ => detail.result.clone(),
        };
    }
}

fn extract_message_fields(detail: &mut MssqlDetail, message: &str) {
    if detail.user.is_empty() {
        for marker in ["Login failed for user '", "Login succeeded for user '"] {
            if let Some(value) = text_between(message, marker, "'") {
                detail.user = value.to_owned();
                break;
            }
        }
    }

    if detail.client_address.is_empty() {
        if let Some(value) = text_between(message, "[CLIENT: ", "]") {
            detail.client_address = value.trim().to_owned();
        }
    }

    if detail.failure_reason.is_empty()
        && let Some(start) = message.find("Reason:")
    {
        let reason = &message[start + "Reason:".len()..];
        let reason = reason
            .split("[CLIENT:")
            .next()
            .unwrap_or(reason)
            .trim()
            .trim_end_matches('.');
        if !reason.is_empty() {
            detail.failure_reason = reason.to_owned();
        }
    }
}

fn text_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let value = &text[text.find(start)? + start.len()..];
    Some(&value[..value.find(end)?])
}

fn matches_name(name: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn copy_if_empty(target: &mut String, value: &str) {
    if target.is_empty() {
        *target = value.to_owned();
    }
}

fn classify(provider: &str, id: u16) -> (&'static str, &'static str) {
    if !is_mssql_provider(provider) {
        return ("未知", "非 MSSQL Provider");
    }
    match id {
        18454 => ("认证成功", "MSSQL: Windows 身份验证登录成功"),
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
    fn parses_mssql_and_keeps_variable_fields_in_details() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQL$SQLEXPRESS"/><EventID>18456</EventID><TimeCreated SystemTime="2026-08-13T10:00:00Z"/></System><EventData><Data Name="LoginName">sa</Data><Data Name="ClientAddress">10.0.0.8</Data><Data Name="FailureReason">密码不匹配</Data><Data Name="State">8</Data></EventData></Event>"#,
        );
        assert!(event.include());
        let values = event.fields();
        assert!(values.iter().any(|(name, value)| *name == "数据库实例"
            && matches!(value, CellValue::Text(value) if value == "SQLEXPRESS")));
        assert_text_field(&values, "结果", "失败");
        assert_text_field(&values, "用户", "sa");
        assert_text_field(&values, "客户端地址", "10.0.0.8");
        assert_text_field(&values, "失败原因", "密码不匹配");
        assert!(values.iter().any(|(name, value)| *name == "详细信息" && matches!(value, CellValue::Text(value) if value.contains("LoginName: sa") && value.contains("State: 8"))));
    }

    #[test]
    fn extracts_login_fields_from_standard_message() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQLSERVER"/><EventID>18456</EventID></System><EventData><Data>Login failed for user 'sa'. Reason: Password did not match that for the login provided. [CLIENT: 192.168.1.8]</Data><Data>18456</Data><Data>14</Data><Data>8</Data></EventData></Event>"#,
        );
        let values = event.fields();

        assert_text_field(&values, "用户", "sa");
        assert_text_field(&values, "客户端地址", "192.168.1.8");
        assert_text_field(
            &values,
            "失败原因",
            "Password did not match that for the login provided",
        );
    }

    #[test]
    fn parses_audit_user_client_and_result() {
        let event = parse(
            r#"<Event><System><Provider Name="MSSQLSERVER"/><EventID>33205</EventID></System><EventData><Data Name="session_server_principal_name">DOMAIN\analyst</Data><Data Name="client_ip">10.0.0.20</Data><Data Name="succeeded">false</Data></EventData></Event>"#,
        );
        let values = event.fields();

        assert_text_field(&values, "结果", "失败");
        assert_text_field(&values, "用户", r"DOMAIN\analyst");
        assert_text_field(&values, "客户端地址", "10.0.0.20");
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
