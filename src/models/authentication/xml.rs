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

    // 用静态字符串标记当前 Data Name，匹配时直接比较 &[u8]
    let mut current_data_name_bytes: &'static [u8] = b"";
    let mut current_extra_name = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            // ── TimeCreated SystemTime ──────────────────────────────────
            Ok(XmlEvent::Start(ref e)) | Ok(XmlEvent::Empty(ref e))
                if e.name().as_ref() == b"TimeCreated" =>
            {
                if let Some(time) = system_time(e) {
                    detail.time = time;
                }
            }

            // ── EventID ────────────────────────────────────────────────
            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"EventID" => {
                if let Ok(text) = reader.read_text(e.name()) {
                    if let Ok(id) = text.parse::<u16>() {
                        detail.event_id = id;
                        detail.description = EventId(id).description();
                    }
                }
            }

            // ── Data Name="..." ────────────────────────────────────────
            // 只记录我们关心的字段名，其余直接忽略，不做 String 分配
            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"Data" => {
                current_data_name_bytes = b""; // 先清空
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"Name" {
                        // 直接在字节层面匹配，零拷贝
                        current_data_name_bytes = match attr.value.as_ref() {
                            b"SubjectUserName" => b"SubjectUserName",
                            b"TargetUserName" => b"TargetUserName",
                            b"IpAddress" => b"IpAddress",
                            b"LogonType" => b"LogonType",
                            b"Status" => b"Status",
                            b"SubStatus" => b"SubStatus",
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

            // ── Text 内容 ──────────────────────────────────────────────
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
                        b"SubStatus" => {
                            if detail.status.is_empty() {
                                detail.status = text.to_owned();
                            }
                        }
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

            // ── End ────────────────────────────────────────────────────
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
