use super::structs::PowerShellDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = PowerShellDetail {
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
                if let Ok(text) = reader.read_text(e.name()) {
                    if let Ok(id) = text.parse::<u16>() {
                        detail.event_id = id;
                        detail.description = EventId(id).description();
                    }
                }
            }

            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"Data" => {
                current_data_name_bytes = b"";
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"Name" {
                        current_data_name_bytes = match attr.value.as_ref() {
                            // Operational.evtx (4103/4104)
                            b"UserId" => b"UserId",
                            b"HostName" => b"HostName",
                            b"ScriptBlockText" => b"ScriptBlockText", // 4104 核心
                            b"MessageNumber" => b"MessageNumber",     // 4104 分片序号
                            // 4103 命令管道
                            b"ContextInfo" => b"ContextInfo",
                            // Windows PowerShell.evtx (400/403/600)
                            b"HostApplication" => b"HostApplication", // 启动命令行
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
                        b"UserId" => detail.user_name = text.to_owned(),
                        b"HostName" => detail.host_name = text.to_owned(),
                        b"ScriptBlockText" => detail.script_block = text.to_owned(),
                        b"MessageNumber" => detail.sequence_id = text.to_owned(),
                        b"ContextInfo" => {
                            if detail.command_line.is_empty() {
                                detail.command_line = text.to_owned();
                            }
                        }
                        b"HostApplication" => {
                            if detail.command_line.is_empty() {
                                detail.command_line = text.to_owned();
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
