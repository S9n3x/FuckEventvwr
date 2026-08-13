use super::structs::ScheduledTaskDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = ScheduledTaskDetail {
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
                            // Security.evtx (4698-4702)
                            b"TaskName" => b"TaskName",
                            b"SubjectUserName" => b"SubjectUserName",
                            b"TaskContent" => b"TaskContent", // 任务 XML 里含 action
                            // TaskScheduler/Operational (200/201)
                            b"ActionName" => b"ActionName", // 实际执行的命令
                            b"ResultCode" => b"ResultCode", // 201 执行结果
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
                        b"TaskName" => detail.task_name = text.to_owned(),
                        b"SubjectUserName" => detail.subject_user_name = text.to_owned(),
                        b"ActionName" => detail.action = text.to_owned(),
                        b"TaskContent" => {
                            // TaskContent 是完整 XML，仅在 action 为空时作兜底
                            if detail.action.is_empty() {
                                detail.action = text.to_owned();
                            }
                        }
                        b"ResultCode" => detail.result_code = text.to_owned(),
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
