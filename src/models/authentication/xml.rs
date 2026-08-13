use super::structs::AuthenticationDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = AuthenticationDetail {
        raw_data: String::new(),
        ..Default::default()
    };
    let mut buf = Vec::with_capacity(512);

    let mut current_data_name_bytes: &'static [u8] = b"";
    let mut current_extra_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref e)) | Ok(XmlEvent::Empty(ref e))
                if e.name().as_ref() == b"TimeCreated" =>
            {
                if let Some(time) = system_time(e) {
                    detail.time = time;
                }
            }

            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"EventID" => {
                if let Ok(text) = reader.read_text(e.name())
                    && let Ok(id) = text.parse::<u16>()
                {
                    detail.event_id = id;
                    detail.description = EventId(id).description();
                }
            }

            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"Data" => {
                current_data_name_bytes = b""; // 先清空
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"Name" {
                        current_data_name_bytes = match attr.value.as_ref() {
                            b"SubjectUserName" => b"SubjectUserName",
                            b"TargetUserName" => b"TargetUserName",
                            b"IpAddress" => b"IpAddress",
                            b"LogonType" => b"LogonType",
                            b"Status" => b"Status",
                            other => {
                                current_extra_name.clear();
                                current_extra_name
                                    .push_str(std::str::from_utf8(other).unwrap_or(""));
                                b""
                            }
                        };
                        break;
                    }
                }
            }

            Ok(XmlEvent::Text(ref e)) => {
                if current_data_name_bytes.is_empty() && current_extra_name.is_empty() {
                    buf.clear();
                    continue;
                }

                if let Ok(cow) = e.decode() {
                    let text = cow.as_ref();

                    if text == "-" || text.is_empty() {
                        current_data_name_bytes = b"";
                        current_extra_name.clear();
                        buf.clear();
                        continue;
                    }

                    match current_data_name_bytes {
                        b"SubjectUserName" => detail.subject_user_name = text.to_owned(),
                        b"TargetUserName" => detail.target_user_name = text.to_owned(),
                        b"IpAddress" => detail.ip_address = text.to_owned(),
                        b"LogonType" => detail.logon_type = text.parse().ok(),
                        b"Status" => detail.status = text.to_owned(),
                        _ => {}
                    }

                    if !current_extra_name.is_empty() {
                        detail.raw_data.push_str(&current_extra_name);
                        detail.raw_data.push_str(": ");
                        detail.raw_data.push_str(text);
                        detail.raw_data.push('\n');
                    }
                }
                current_data_name_bytes = b"";
                current_extra_name.clear();
            }

            Ok(XmlEvent::End(ref e)) => match e.name().as_ref() {
                b"Data" => {
                    current_data_name_bytes = b"";
                    current_extra_name.clear();
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

    Box::new(detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::definition::CellValue;

    #[test]
    fn keeps_sub_status_in_details_without_adding_a_sparse_column() {
        let event = parse(
            r#"<Event><System><EventID>4625</EventID></System><EventData><Data Name="Status">0xc000006d</Data><Data Name="SubStatus">0xc000006a</Data></EventData></Event>"#,
        );
        let fields = event.fields();

        assert_text_field(&fields, "状态码", "0xc000006d");
        assert!(!fields.iter().any(|(name, _)| *name == "子状态码"));
        assert!(fields.iter().any(|(name, value)| {
            *name == "详细信息"
                && matches!(value, CellValue::Text(value) if value.contains("SubStatus: 0xc000006a"))
        }));
    }

    fn assert_text_field(
        fields: &[(&'static str, CellValue<'_>)],
        expected_name: &str,
        expected_value: &str,
    ) {
        assert!(fields.iter().any(|(name, value)| {
            *name == expected_name
                && matches!(value, CellValue::Text(value) if value == expected_value)
        }));
    }
}
