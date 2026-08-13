use super::structs::SessionDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = SessionDetail::default();
    let mut buf = Vec::with_capacity(512);
    let mut current_data_name = String::new();
    let mut in_payload = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref element)) | Ok(XmlEvent::Empty(ref element))
                if element.name().as_ref() == b"Provider" =>
            {
                for attribute in element.attributes().flatten() {
                    if attribute.key.as_ref() == b"Name" {
                        if let Ok(value) = attribute.unescape_value() {
                            detail.provider = value.into_owned();
                        }
                        break;
                    }
                }
            }
            Ok(XmlEvent::Start(ref element)) | Ok(XmlEvent::Empty(ref element))
                if element.name().as_ref() == b"TimeCreated" =>
            {
                if let Some(time) = system_time(element) {
                    detail.time = time;
                }
            }
            Ok(XmlEvent::Start(ref element)) if element.name().as_ref() == b"EventID" => {
                if let Ok(text) = reader.read_text(element.name())
                    && let Ok(event_id) = text.parse::<u16>()
                {
                    detail.event_id = event_id;
                    detail.description = EventId(event_id).description();
                }
            }
            Ok(XmlEvent::Start(ref element))
                if matches!(element.name().as_ref(), b"EventData" | b"UserData") =>
            {
                in_payload = true;
            }
            Ok(XmlEvent::Start(ref element)) if in_payload => {
                current_data_name.clear();
                if element.name().as_ref() == b"Data" {
                    for attribute in element.attributes().flatten() {
                        if attribute.key.as_ref() == b"Name" {
                            current_data_name.push_str(
                                std::str::from_utf8(attribute.value.as_ref()).unwrap_or_default(),
                            );
                            break;
                        }
                    }
                } else if local_name(element.name().as_ref()) != "EventXML" {
                    current_data_name.push_str(local_name(element.name().as_ref()));
                }
            }
            Ok(XmlEvent::Text(ref element)) if !current_data_name.is_empty() => {
                if let Ok(value) = element.decode() {
                    let value = value.trim();
                    if !value.is_empty()
                        && value != "-"
                        && !assign_field(&mut detail, &current_data_name, value)
                    {
                        append_raw(&mut detail.raw_data, &current_data_name, value);
                    }
                }
            }
            Ok(XmlEvent::End(ref element)) => match element.name().as_ref() {
                b"Data" => current_data_name.clear(),
                b"EventData" | b"UserData" => {
                    in_payload = false;
                    current_data_name.clear();
                }
                b"Event" => break,
                _ if in_payload => current_data_name.clear(),
                _ => {}
            },
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if detail.raw_data.ends_with('\n') {
        detail.raw_data.pop();
    }
    detail.include = is_session_event(&detail.provider, detail.event_id);

    Box::new(detail)
}

fn is_session_event(provider: &str, event_id: u16) -> bool {
    match provider {
        "Microsoft-Windows-Security-Auditing" => matches!(event_id, 4634 | 4647),
        "Microsoft-Windows-TerminalServices-LocalSessionManager" => {
            matches!(event_id, 21 | 22 | 23 | 24 | 25 | 40)
        }
        "Microsoft-Windows-TerminalServices-RemoteConnectionManager" => event_id == 1149,
        _ => false,
    }
}

fn local_name(name: &[u8]) -> &str {
    let name = std::str::from_utf8(name).unwrap_or_default();
    name.rsplit(':').next().unwrap_or(name)
}

fn assign_field(detail: &mut SessionDetail, name: &str, value: &str) -> bool {
    match (detail.event_id, name) {
        (4634 | 4647, "SubjectUserName" | "TargetUserName") => detail.user_name = value.to_owned(),
        (4634 | 4647, "SubjectDomainName" | "TargetDomainName") => {
            detail.domain_name = value.to_owned()
        }
        (4634 | 4647, "SubjectLogonId" | "TargetLogonId") => detail.session_id = value.to_owned(),
        (4634, "LogonType") => detail.logon_type = value.parse().ok(),

        (21..=25, "User") => assign_qualified_user(detail, value),
        (21 | 22 | 23 | 24 | 25 | 40, "SessionID") | (40, "Session") => {
            detail.session_id = value.to_owned()
        }
        (21 | 22 | 24 | 25, "Address") => detail.remote_address = value.to_owned(),
        (40, "Reason") => detail.reason = value.to_owned(),

        (1149, "Param1") => detail.user_name = value.to_owned(),
        (1149, "Param2") => detail.domain_name = value.to_owned(),
        (1149, "Param3") => detail.remote_address = value.to_owned(),

        (_, "User") if detail.user_name.is_empty() => assign_qualified_user(detail, value),
        (_, "SessionID" | "Session") if detail.session_id.is_empty() => {
            detail.session_id = value.to_owned()
        }
        (_, "Address" | "IpAddress") if detail.remote_address.is_empty() => {
            detail.remote_address = value.to_owned()
        }
        (_, "Reason") if detail.reason.is_empty() => detail.reason = value.to_owned(),
        _ => return false,
    }
    true
}

fn assign_qualified_user(detail: &mut SessionDetail, value: &str) {
    if let Some((domain, user)) = value.split_once('\\') {
        detail.domain_name = domain.to_owned();
        detail.user_name = user.to_owned();
    } else {
        detail.user_name = value.to_owned();
    }
}

fn append_raw(raw: &mut String, name: &str, value: &str) {
    raw.push_str(name);
    raw.push_str(": ");
    raw.push_str(value);
    raw.push('\n');
}
