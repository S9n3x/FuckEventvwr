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
    let mut unnamed_data_index = 0usize;

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
                unnamed_data_index += 1;
                let name = e
                    .attributes()
                    .flatten()
                    .find(|attribute| attribute.key.as_ref() == b"Name")
                    .and_then(|attribute| attribute.unescape_value().ok())
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|| format!("Data{unnamed_data_index}"));

                if let Ok(value) = reader.read_text(e.name()) {
                    let value = value.trim();
                    if !value.is_empty() && value != "-" {
                        assign_data(&mut detail, &name, value);
                    }
                }
            }

            Ok(XmlEvent::End(ref e)) if e.name().as_ref() == b"Event" => break,

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

fn assign_data(detail: &mut PowerShellDetail, name: &str, value: &str) {
    match name {
        "ScriptBlockText" => detail.script_block = value.to_owned(),
        "HostApplication" if detail.command_line.is_empty() => {
            detail.command_line = value.to_owned()
        }
        _ => {
            if name == "ContextInfo" || name.starts_with("Data") {
                extract_host_application(detail, value);
            }
            append_raw(&mut detail.raw_data, name, value);
        }
    }
}

fn extract_host_application(detail: &mut PowerShellDetail, value: &str) {
    if !detail.command_line.is_empty() {
        return;
    }

    let Some(marker) = value.find("HostApplication") else {
        return;
    };
    let remainder = value[marker + "HostApplication".len()..].trim_start();
    let Some(remainder) = remainder.strip_prefix('=') else {
        return;
    };
    let remainder = remainder.trim_start();
    let end = ["\r", "\n", "&#10;", "&#xA;", "&#x0A;"]
        .into_iter()
        .filter_map(|separator| remainder.find(separator))
        .min()
        .unwrap_or(remainder.len());
    let command_line = remainder[..end].trim();
    if !command_line.is_empty() {
        detail.command_line = command_line.to_owned();
    }
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
    fn extracts_classic_command_line_and_preserves_original_payload() {
        let event = parse(
            r#"<Event><System><EventID>400</EventID></System><EventData><Data>Available</Data><Data>None</Data><Data>HostName=ConsoleHost&#10;HostApplication=powershell.exe -File audit.ps1&#10;RunspaceId=runspace-id</Data></EventData></Event>"#,
        );
        let fields = event.fields();
        let raw = text_field(&fields, "详细信息");

        assert_eq!(
            text_field(&fields, "命令行"),
            "powershell.exe -File audit.ps1"
        );
        assert!(raw.contains("Data1: Available"));
        assert!(raw.contains("Data2: None"));
        assert!(raw.contains("HostApplication=powershell.exe -File audit.ps1"));
        assert!(raw.contains("RunspaceId=runspace-id"));
        assert_sparse_columns_removed(&fields);
    }

    #[test]
    fn extracts_host_application_from_4103_context_and_preserves_payload() {
        let event = parse(
            r#"<Event><System><EventID>4103</EventID></System><EventData><Data Name="ContextInfo">HostName = ConsoleHost&#10;HostApplication = powershell.exe -NoProfile</Data><Data Name="UserData"></Data><Data Name="Payload">CommandInvocation(Get-Item): "Get-Item"</Data></EventData></Event>"#,
        );
        let fields = event.fields();
        let raw = text_field(&fields, "详细信息");

        assert_eq!(text_field(&fields, "命令行"), "powershell.exe -NoProfile");
        assert!(raw.contains("ContextInfo: HostName = ConsoleHost"));
        assert!(raw.contains("Payload: CommandInvocation(Get-Item)"));
    }

    #[test]
    fn keeps_sparse_4104_metadata_in_details() {
        let event = parse(
            r#"<Event><System><EventID>4104</EventID></System><EventData><Data Name="MessageNumber">2</Data><Data Name="MessageTotal">3</Data><Data Name="ScriptBlockText">Get-Process</Data><Data Name="ScriptBlockId">11111111-2222-3333-4444-555555555555</Data><Data Name="Path">C:\audit.ps1</Data></EventData></Event>"#,
        );
        let fields = event.fields();
        let raw = text_field(&fields, "详细信息");

        assert_eq!(text_field(&fields, "脚本内容"), "Get-Process");
        assert!(raw.contains("MessageNumber: 2"));
        assert!(raw.contains("MessageTotal: 3"));
        assert!(raw.contains("ScriptBlockId: 11111111-2222-3333-4444-555555555555"));
        assert!(raw.contains(r"Path: C:\audit.ps1"));
        assert_sparse_columns_removed(&fields);
    }

    #[test]
    fn keeps_4105_and_4106_correlation_fields_in_details() {
        for id in [4105, 4106] {
            let xml = format!(
                r#"<Event><System><EventID>{id}</EventID></System><EventData><Data Name="ScriptBlockId">block-id</Data><Data Name="RunspaceId">runspace-id</Data></EventData></Event>"#
            );
            let event = parse(&xml);
            let fields = event.fields();
            let raw = text_field(&fields, "详细信息");

            assert!(raw.contains("ScriptBlockId: block-id"));
            assert!(raw.contains("RunspaceId: runspace-id"));
            assert_sparse_columns_removed(&fields);
        }
    }

    fn assert_sparse_columns_removed(fields: &[(&'static str, CellValue<'_>)]) {
        for name in [
            "用户",
            "PS宿主",
            "分片序号",
            "分片总数",
            "脚本块ID",
            "脚本路径",
            "运行空间ID",
        ] {
            assert!(!fields.iter().any(|(field_name, _)| *field_name == name));
        }
    }

    fn text_field<'a>(fields: &'a [(&'static str, CellValue<'a>)], name: &str) -> &'a str {
        fields
            .iter()
            .find_map(|(field_name, value)| match value {
                CellValue::Text(value) if *field_name == name => Some(value.as_ref()),
                _ => None,
            })
            .unwrap_or("")
    }
}
