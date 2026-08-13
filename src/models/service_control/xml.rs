use super::structs::ServiceControlDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = ServiceControlDetail::default();
    let mut buf = Vec::with_capacity(512);
    let mut current_data_name = String::new();
    let mut in_event_data = false;

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
            Ok(XmlEvent::Start(ref element)) if element.name().as_ref() == b"EventData" => {
                in_event_data = true;
            }
            Ok(XmlEvent::Start(ref element)) if in_event_data => {
                current_data_name.clear();
                if element.name().as_ref() == b"Data" {
                    for attribute in element.attributes().flatten() {
                        if attribute.key.as_ref() == b"Name" {
                            if let Ok(value) = attribute.unescape_value() {
                                current_data_name = value.into_owned();
                            }
                            break;
                        }
                    }
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
                b"EventData" => {
                    in_event_data = false;
                    current_data_name.clear();
                }
                b"Event" => break,
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
    detail.include = detail.provider == "Service Control Manager"
        && matches!(detail.event_id, 7034 | 7035 | 7036 | 7040 | 7045);

    Box::new(detail)
}

fn assign_field(detail: &mut ServiceControlDetail, name: &str, value: &str) -> bool {
    match (detail.event_id, name) {
        (7034, "param1" | "ServiceName") => detail.service_name = value.to_owned(),

        (7035, "param1" | "ServiceName") => detail.service_name = value.to_owned(),

        (7036, "param1" | "ServiceName") => detail.service_name = value.to_owned(),

        (7040, "param1" | "ServiceDisplayName") => detail.service_name = value.to_owned(),
        (7040, "param2" | "OldStartType") => detail.old_start_type = value.to_owned(),
        (7040, "param3" | "NewStartType") => detail.new_start_type = value.to_owned(),
        (7040, "param4" | "ServiceName") => detail.internal_service_name = value.to_owned(),

        (7045, "ServiceName") => detail.service_name = value.to_owned(),
        (7045, "ImagePath") => detail.image_path = value.to_owned(),
        (7045, "ServiceType") => detail.service_type = value.to_owned(),
        (7045, "StartType") => detail.start_type = value.to_owned(),

        _ => return false,
    }
    true
}

fn append_raw(raw: &mut String, name: &str, value: &str) {
    raw.push_str(name);
    raw.push_str(": ");
    raw.push_str(value);
    raw.push('\n');
}
