use super::structs::ScheduledTaskDetail;
use crate::cfg::event::EventId;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::{BytesStart, Event as XmlEvent};
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = ScheduledTaskDetail::default();
    let mut buf = Vec::with_capacity(512);

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref element)) | Ok(XmlEvent::Empty(ref element))
                if element.name().as_ref() == b"Provider" =>
            {
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
                if let Ok(text) = reader.read_text(element.name())
                    && let Ok(event_id) = text.parse::<u16>()
                {
                    detail.event_id = event_id;
                    detail.description = EventId(event_id).description();
                }
            }
            Ok(XmlEvent::Start(ref element)) if element.name().as_ref() == b"Data" => {
                let mut name = String::new();
                read_attribute(element, b"Name", &mut name);
                if let Ok(value) = reader.read_text(element.name()) {
                    let value = quick_xml::escape::unescape(value.as_ref())
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|_| value.into_owned());
                    if !value.is_empty() && value != "-" {
                        assign_field(&mut detail, &name, &value);
                    }
                }
            }
            Ok(XmlEvent::End(ref element)) if element.name().as_ref() == b"Event" => break,
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if detail.raw_data.ends_with('\n') {
        detail.raw_data.pop();
    }
    detail.include = is_scheduled_task_event(&detail.provider, detail.event_id);

    Box::new(detail)
}

fn assign_field(detail: &mut ScheduledTaskDetail, name: &str, value: &str) {
    match name {
        "TaskName" => detail.task_name = value.to_owned(),
        "SubjectUserName" | "UserContext" | "UserName" => {
            detail.subject_user_name = value.to_owned()
        }
        "ActionName" => detail.action = value.to_owned(),
        "ResultCode" => detail.result_code = value.to_owned(),
        "TaskContent" | "TaskContentNew" => {
            let actions = parse_task_actions(value);
            if !actions.is_empty() {
                detail.action = actions.join("\n");
            }
            append_raw(&mut detail.raw_data, name, value);
        }
        _ => append_raw(&mut detail.raw_data, name, value),
    }
}

fn is_scheduled_task_event(provider: &str, event_id: u16) -> bool {
    match provider {
        "Microsoft-Windows-Security-Auditing" => matches!(event_id, 4698..=4702),
        "Microsoft-Windows-TaskScheduler" => matches!(event_id, 106 | 140 | 141 | 200 | 201),
        _ => false,
    }
}

#[derive(Default)]
struct TaskAction {
    kind: String,
    command: String,
    arguments: String,
    working_directory: String,
    class_id: String,
    data: String,
    to: String,
    subject: String,
    title: String,
    body: String,
}

impl TaskAction {
    fn render(self) -> String {
        match self.kind.as_str() {
            "Exec" => {
                let mut value = self.command;
                if !self.arguments.is_empty() {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(&self.arguments);
                }
                if !self.working_directory.is_empty() {
                    value.push_str(" [WorkingDirectory: ");
                    value.push_str(&self.working_directory);
                    value.push(']');
                }
                value
            }
            "ComHandler" => join_action_parts("COM", &self.class_id, "Data", &self.data),
            "SendEmail" => join_action_parts("Email", &self.to, "Subject", &self.subject),
            "ShowMessage" => join_action_parts("Message", &self.title, "Body", &self.body),
            _ => String::new(),
        }
    }
}

fn join_action_parts(kind: &str, primary: &str, secondary_name: &str, secondary: &str) -> String {
    let mut value = kind.to_owned();
    if !primary.is_empty() {
        value.push_str(": ");
        value.push_str(primary);
    }
    if !secondary.is_empty() {
        value.push_str(" [");
        value.push_str(secondary_name);
        value.push_str(": ");
        value.push_str(secondary);
        value.push(']');
    }
    value
}

fn parse_task_actions(task_xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(task_xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::with_capacity(512);
    let mut in_actions = false;
    let mut current_field = String::new();
    let mut current_action: Option<TaskAction> = None;
    let mut actions = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref element)) => {
                let qualified_name = element.name();
                let name = local_name(qualified_name.as_ref());
                if name == "Actions" {
                    in_actions = true;
                } else if in_actions && is_action_name(name) {
                    current_action = Some(TaskAction {
                        kind: name.to_owned(),
                        ..Default::default()
                    });
                } else if current_action.is_some() {
                    current_field.clear();
                    current_field.push_str(name);
                }
            }
            Ok(XmlEvent::Text(ref element)) if current_action.is_some() => {
                if let Ok(value) = element.decode() {
                    assign_action_field(
                        current_action.as_mut().expect("action checked above"),
                        &current_field,
                        value.as_ref(),
                    );
                }
            }
            Ok(XmlEvent::End(ref element)) => {
                let qualified_name = element.name();
                let name = local_name(qualified_name.as_ref());
                if is_action_name(name) {
                    if let Some(action) = current_action.take() {
                        let rendered = action.render();
                        if !rendered.is_empty() {
                            actions.push(rendered);
                        }
                    }
                } else if name == "Actions" {
                    in_actions = false;
                }
                current_field.clear();
            }
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    actions
}

fn is_action_name(name: &str) -> bool {
    matches!(name, "Exec" | "ComHandler" | "SendEmail" | "ShowMessage")
}

fn assign_action_field(action: &mut TaskAction, name: &str, value: &str) {
    match name {
        "Command" => action.command = value.to_owned(),
        "Arguments" => action.arguments = value.to_owned(),
        "WorkingDirectory" => action.working_directory = value.to_owned(),
        "ClassId" => action.class_id = value.to_owned(),
        "Data" => action.data = value.to_owned(),
        "To" => action.to = value.to_owned(),
        "Subject" => action.subject = value.to_owned(),
        "Title" => action.title = value.to_owned(),
        "Body" => action.body = value.to_owned(),
        _ => {}
    }
}

fn read_attribute(element: &BytesStart<'_>, name: &[u8], target: &mut String) {
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
    fn extracts_exec_actions_from_security_task_content() {
        let event = parse(
            r#"<Event><System><Provider Name="Microsoft-Windows-Security-Auditing"/><EventID>4698</EventID></System><EventData><Data Name="SubjectUserName">analyst</Data><Data Name="TaskName">\Audit</Data><Data Name="TaskContent">&lt;Task xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task"&gt;&lt;Actions&gt;&lt;Exec&gt;&lt;Command&gt;powershell.exe&lt;/Command&gt;&lt;Arguments&gt;-File audit.ps1&lt;/Arguments&gt;&lt;WorkingDirectory&gt;C:\Tools&lt;/WorkingDirectory&gt;&lt;/Exec&gt;&lt;Exec&gt;&lt;Command&gt;cmd.exe&lt;/Command&gt;&lt;Arguments&gt;/c whoami&lt;/Arguments&gt;&lt;/Exec&gt;&lt;/Actions&gt;&lt;/Task&gt;</Data></EventData></Event>"#,
        );

        assert!(event.include());
        assert_eq!(text_field(event.as_ref(), "操作者"), "analyst");
        assert_eq!(
            text_field(event.as_ref(), "执行动作"),
            "powershell.exe -File audit.ps1 [WorkingDirectory: C:\\Tools]\ncmd.exe /c whoami"
        );
        assert!(text_field(event.as_ref(), "详细信息").contains("TaskContent: <Task"));
    }

    #[test]
    fn recognizes_updated_task_content_and_operational_users() {
        let updated = parse(
            r#"<Event><System><Provider Name="Microsoft-Windows-Security-Auditing"/><EventID>4702</EventID></System><EventData><Data Name="TaskName">\Updated</Data><Data Name="TaskContentNew">&lt;Task&gt;&lt;Actions&gt;&lt;ComHandler&gt;&lt;ClassId&gt;{00000000-0000-0000-0000-000000000001}&lt;/ClassId&gt;&lt;Data&gt;payload&lt;/Data&gt;&lt;/ComHandler&gt;&lt;/Actions&gt;&lt;/Task&gt;</Data></EventData></Event>"#,
        );
        assert_eq!(
            text_field(updated.as_ref(), "执行动作"),
            "COM: {00000000-0000-0000-0000-000000000001} [Data: payload]"
        );

        let registered = parse(
            r#"<Event><System><Provider Name="Microsoft-Windows-TaskScheduler"/><EventID>106</EventID></System><EventData><Data Name="TaskName">\Registered</Data><Data Name="UserContext">DOMAIN\operator</Data></EventData></Event>"#,
        );
        assert_eq!(
            text_field(registered.as_ref(), "操作者"),
            r"DOMAIN\operator"
        );

        let changed = parse(
            r#"<Event><System><Provider Name="Microsoft-Windows-TaskScheduler"/><EventID>140</EventID></System><EventData><Data Name="TaskName">\Changed</Data><Data Name="UserName">DOMAIN\admin</Data></EventData></Event>"#,
        );
        assert_eq!(text_field(changed.as_ref(), "操作者"), r"DOMAIN\admin");
    }

    #[test]
    fn reads_operational_action_and_result_code() {
        let event = parse(
            r#"<Event><System><Provider Name="Microsoft-Windows-TaskScheduler"/><EventID>201</EventID></System><EventData><Data Name="TaskName">\Audit</Data><Data Name="TaskInstanceId">instance</Data><Data Name="ActionName">C:\Tools\audit.exe</Data><Data Name="ResultCode">0</Data></EventData></Event>"#,
        );

        assert!(event.include());
        assert_eq!(
            text_field(event.as_ref(), "执行动作"),
            r"C:\Tools\audit.exe"
        );
        assert_eq!(text_field(event.as_ref(), "结果码"), "0");
        assert!(text_field(event.as_ref(), "详细信息").contains("TaskInstanceId: instance"));
    }

    #[test]
    fn rejects_same_event_id_from_another_provider() {
        let event = parse(
            r#"<Event><System><Provider Name="Other"/><EventID>4698</EventID></System><EventData><Data Name="TaskName">\Wrong</Data></EventData></Event>"#,
        );
        assert!(!event.include());
    }

    fn text_field(event: &(dyn EventRecord + Send), name: &str) -> String {
        event
            .fields()
            .into_iter()
            .find_map(|(field_name, value)| match value {
                CellValue::Text(value) if field_name == name => Some(value.into_owned()),
                _ => None,
            })
            .unwrap_or_default()
    }
}
