use super::structs::ApplicationDetail;
use crate::parser::definition::EventRecord;
use crate::parser::utils::system_time;
use quick_xml::events::Event as XmlEvent;
use quick_xml::reader::Reader;

pub fn parse(xml: &str) -> Box<dyn EventRecord + Send> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut detail = ApplicationDetail::default();
    let mut buf = Vec::with_capacity(512);
    let mut in_payload = false;
    let mut current_name = String::new();
    let mut data_index = 0usize;
    let mut positional_data = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"Provider" => {
                read_attribute(e, b"Name", &mut detail.provider);
            }
            Ok(XmlEvent::Empty(ref e)) if e.name().as_ref() == b"Provider" => {
                read_attribute(e, b"Name", &mut detail.provider);
            }
            Ok(XmlEvent::Start(ref e)) | Ok(XmlEvent::Empty(ref e))
                if e.name().as_ref() == b"TimeCreated" =>
            {
                if let Some(time) = system_time(e) {
                    detail.time = time;
                }
            }
            Ok(XmlEvent::Start(ref e)) if e.name().as_ref() == b"EventID" => {
                if let Ok(text) = reader.read_text(e.name()) {
                    detail.event_id = text.parse::<u16>().unwrap_or_default();
                }
            }
            Ok(XmlEvent::Start(ref e))
                if matches!(e.name().as_ref(), b"EventData" | b"UserData") =>
            {
                in_payload = true;
            }
            Ok(XmlEvent::Start(ref e)) if in_payload => {
                current_name.clear();
                if e.name().as_ref() == b"Data" {
                    positional_data.push(String::new());
                    data_index = positional_data.len() - 1;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Name" {
                            current_name.push_str(
                                std::str::from_utf8(attr.value.as_ref()).unwrap_or_default(),
                            );
                            break;
                        }
                    }
                } else {
                    current_name.push_str(local_name(e.name().as_ref()));
                }
            }
            Ok(XmlEvent::Text(ref e)) if in_payload => {
                if let Ok(value) = e.decode() {
                    let value = value.trim();
                    if !value.is_empty() && value != "-" {
                        if current_name.is_empty() {
                            positional_data[data_index] = value.to_owned();
                            append_raw(&mut detail.raw_data, &format!("Data{data_index}"), value);
                        } else {
                            assign_named(&mut detail, &current_name, value);
                            if !is_known_name(&current_name) {
                                append_raw(&mut detail.raw_data, &current_name, value);
                            }
                        }
                    }
                }
            }
            Ok(XmlEvent::End(ref e)) => match e.name().as_ref() {
                b"Data" => current_name.clear(),
                b"EventData" | b"UserData" => {
                    in_payload = false;
                    current_name.clear();
                }
                b"Event" => break,
                _ if in_payload => current_name.clear(),
                _ => {}
            },
            Ok(XmlEvent::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    detail.include = classify_and_map(&mut detail, &positional_data);
    let (event_type, description) =
        classify(&detail.provider, detail.event_id, &detail.report_type);
    detail.event_type = event_type.to_owned();
    detail.description = description;
    if detail.raw_data.ends_with('\n') {
        detail.raw_data.pop();
    }

    Box::new(detail)
}

fn classify(provider: &str, id: u16, report_type: &str) -> (&'static str, &'static str) {
    match (provider, id, report_type) {
        ("Application Error", 1000, _) => ("崩溃", "Application Error: 应用程序崩溃"),
        ("Windows Error Reporting", 1001, report) if report.starts_with("AppHang") => {
            ("卡死报告", "Windows Error Reporting: 程序卡死报告")
        }
        ("Windows Error Reporting", 1001, report) if is_crash_report(report) => {
            ("崩溃报告", "Windows Error Reporting: 程序崩溃报告")
        }
        ("Application Hang", 1002, _) => ("卡死", "Application Hang: 应用程序无响应"),
        (".NET Runtime", 1026, _) => ("运行时异常", ".NET Runtime: 未处理的运行时异常"),
        ("MsiInstaller", 1033 | 11707, _) => ("安装", "MsiInstaller: 产品安装成功"),
        ("MsiInstaller", 11708, _) => ("安装失败", "MsiInstaller: 产品安装失败"),
        ("MsiInstaller", 1034 | 11724, _) => ("卸载", "MsiInstaller: 产品卸载完成"),
        ("MsiInstaller", 1040, _) => ("安装过程", "MsiInstaller: 安装事务开始"),
        ("MsiInstaller", 1042, _) => ("安装过程", "MsiInstaller: 安装事务结束"),
        _ => ("程序事件", "未分类的程序事件"),
    }
}

fn classify_and_map(detail: &mut ApplicationDetail, data: &[String]) -> bool {
    match (detail.provider.as_str(), detail.event_id) {
        ("Application Error", 1000) => {
            map_application_error(detail, data);
            true
        }
        ("Windows Error Reporting", 1001) => map_wer(detail, data),
        ("Application Hang", 1002) => {
            map_application_hang(detail, data);
            true
        }
        (".NET Runtime", 1026)
        | ("MsiInstaller", 1033 | 1034 | 1040 | 1042 | 11707 | 11708 | 11724) => {
            if detail.provider == "MsiInstaller" {
                map_msi(detail, data);
            }
            true
        }
        _ => false,
    }
}

fn map_wer(detail: &mut ApplicationDetail, data: &[String]) -> bool {
    detail.report_type = data.get(2).cloned().unwrap_or_default();
    if detail.report_type.starts_with("AppHang") {
        detail.application_name = data.get(5).cloned().unwrap_or_default();
        detail.application_version = data.get(6).cloned().unwrap_or_default();
        return true;
    }
    if is_crash_report(&detail.report_type) {
        detail.application_name = data.get(5).cloned().unwrap_or_default();
        detail.application_version = data.get(6).cloned().unwrap_or_default();
        detail.fault_module = data.get(8).cloned().unwrap_or_default();
        detail.fault_module_version = data.get(9).cloned().unwrap_or_default();
        detail.exception_code = data.get(11).cloned().unwrap_or_default();
        return true;
    }
    false
}

fn is_crash_report(report_type: &str) -> bool {
    matches!(report_type, "APPCRASH" | "BEX" | "BEX64" | "CLR20r3")
        || report_type.starts_with("APPCRASH")
}

fn map_application_error(detail: &mut ApplicationDetail, data: &[String]) {
    copy_if_present(&mut detail.application_name, data.first());
    copy_if_present(&mut detail.application_version, data.get(1));
    copy_if_present(&mut detail.fault_module, data.get(3));
    copy_if_present(&mut detail.fault_module_version, data.get(4));
    copy_if_present(&mut detail.exception_code, data.get(6));
    copy_if_present(&mut detail.process_id, data.get(8));
    copy_if_present(&mut detail.application_path, data.get(10));
    copy_if_present(&mut detail.fault_module_path, data.get(11));
}

fn copy_if_present(target: &mut String, value: Option<&String>) {
    if target.is_empty() {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            *target = value.clone();
        }
    }
}

fn map_application_hang(detail: &mut ApplicationDetail, data: &[String]) {
    detail.application_name = data.first().cloned().unwrap_or_default();
    detail.application_version = data.get(1).cloned().unwrap_or_default();
    detail.process_id = data.get(2).cloned().unwrap_or_default();
    detail.application_path = data.get(5).cloned().unwrap_or_default();
}

fn map_msi(detail: &mut ApplicationDetail, data: &[String]) {
    match detail.event_id {
        1033 | 1034 => {
            copy_if_present(&mut detail.application_name, data.first());
            copy_if_present(&mut detail.application_version, data.get(1));
            append_raw_if_present(&mut detail.raw_data, "厂商", data.get(4));
        }
        1040 | 1042 => {
            if let Some(identifier) = data
                .first()
                .filter(|value| !value.is_empty() && *value != "(NULL)")
            {
                detail.transaction_identifier = identifier.clone();
                let path = std::path::Path::new(identifier);
                if path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"))
                {
                    detail.application_path = identifier.clone();
                    detail.application_name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default()
                        .to_owned();
                }
            }
        }
        11707 | 11708 | 11724 => {
            if let Some(message) = data
                .first()
                .filter(|value| !value.is_empty() && *value != "(NULL)")
            {
                let message = message.strip_prefix("Product: ").unwrap_or(message);
                if let Some((name, _)) = message.split_once(" -- ") {
                    detail.application_name = name.to_owned();
                } else {
                    detail.application_name = message.to_owned();
                }
            }
        }
        _ => {}
    }
}

fn read_attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8], target: &mut String) {
    for attr in element.attributes().flatten() {
        if attr.key.as_ref() == name {
            if let Ok(value) = attr.unescape_value() {
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

fn append_raw_if_present(raw: &mut String, name: &str, value: Option<&String>) {
    if let Some(value) = value.filter(|value| !value.is_empty() && *value != "(NULL)") {
        append_raw(raw, name, value);
    }
}

fn is_known_name(name: &str) -> bool {
    matches!(
        name,
        "ApplicationName"
            | "Application"
            | "AppName"
            | "P1"
            | "ApplicationVersion"
            | "AppVersion"
            | "P2"
            | "ApplicationPath"
            | "AppPath"
            | "P10"
            | "FaultModuleName"
            | "FaultModule"
            | "ModuleName"
            | "P4"
            | "FaultModuleVersion"
            | "ModuleVersion"
            | "P5"
            | "FaultModulePath"
            | "ModulePath"
            | "ExceptionCode"
            | "P7"
            | "ProcessId"
            | "ProcessID"
            | "P9"
            | "ProductName"
            | "Product"
            | "ProductVersion"
            | "Manufacturer"
            | "IntegratorReportId"
            | "Message"
            | "Description"
            | "ErrorCode"
            | "ReturnValue"
    )
}

fn assign_named(detail: &mut ApplicationDetail, name: &str, value: &str) {
    match name {
        "ApplicationName" | "Application" | "AppName" | "P1" => {
            detail.application_name = value.to_owned()
        }
        "ApplicationVersion" | "AppVersion" | "P2" => detail.application_version = value.to_owned(),
        "ApplicationPath" | "AppPath" | "P10" => detail.application_path = value.to_owned(),
        "FaultModuleName" | "FaultModule" | "ModuleName" | "P4" => {
            detail.fault_module = value.to_owned()
        }
        "FaultModuleVersion" | "ModuleVersion" | "P5" => {
            detail.fault_module_version = value.to_owned()
        }
        "FaultModulePath" | "ModulePath" => detail.fault_module_path = value.to_owned(),
        "ExceptionCode" | "P7" => detail.exception_code = value.to_owned(),
        "ProcessId" | "ProcessID" | "P9" => detail.process_id = value.to_owned(),
        "ProductName" | "Product" => detail.application_name = value.to_owned(),
        "ProductVersion" => detail.application_version = value.to_owned(),
        "Manufacturer" => append_raw(&mut detail.raw_data, name, value),
        "IntegratorReportId" => append_raw(&mut detail.raw_data, name, value),
        "Message" | "Description" | "ErrorCode" | "ReturnValue" => {
            append_raw(&mut detail.raw_data, name, value)
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(xml: &str) -> Vec<(&'static str, String)> {
        parse(xml)
            .fields()
            .into_iter()
            .map(|(name, value)| {
                let value = match value {
                    crate::parser::definition::CellValue::Text(value) => value.into_owned(),
                    crate::parser::definition::CellValue::Number(value) => value.to_string(),
                };
                (name, value)
            })
            .collect()
    }

    fn parsed(xml: &str) -> Box<dyn EventRecord + Send> {
        parse(xml)
    }

    #[test]
    fn parses_application_error_named_data() {
        let values = fields(
            r#"<Event><System><Provider Name="Application Error"/><EventID>1000</EventID><TimeCreated SystemTime="2026-08-13T10:00:00Z"/></System><EventData><Data Name="AppName">demo.exe</Data><Data Name="FaultModuleName">ntdll.dll</Data><Data Name="ExceptionCode">c0000005</Data><Data Name="AppPath">C:\demo.exe</Data></EventData></Event>"#,
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "程序名称" && value == "demo.exe")
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "故障模块" && value == "ntdll.dll")
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "事件类型" && value == "崩溃")
        );
    }

    #[test]
    fn parses_msi_installer_user_data() {
        let values = fields(
            r#"<Event><System><Provider Name="MsiInstaller"/><EventID>11707</EventID></System><UserData><MsiInstaller><ProductName>Saga</ProductName><ProductVersion>0.3.1</ProductVersion><Manufacturer>Synex</Manufacturer><Message>Success</Message></MsiInstaller></UserData></Event>"#,
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "程序名称" && value == "Saga")
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "程序版本" && value == "0.3.1")
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "事件类型" && value == "安装")
        );
    }

    #[test]
    fn parses_msi_installer_positional_data() {
        let installed = fields(
            r#"<Event><System><Provider Name="MsiInstaller"/><EventID>11707</EventID></System><EventData><Data>Product: CC Switch -- Installation completed successfully.</Data><Data>(NULL)</Data></EventData></Event>"#,
        );
        let product = fields(
            r#"<Event><System><Provider Name="MsiInstaller"/><EventID>1033</EventID></System><EventData><Data>CC Switch</Data><Data>3.19.2</Data><Data>1033</Data><Data>0</Data><Data>ccswitch</Data></EventData></Event>"#,
        );
        assert!(
            installed
                .iter()
                .any(|(name, value)| *name == "程序名称" && value == "CC Switch")
        );
        assert!(
            product
                .iter()
                .any(|(name, value)| *name == "程序名称" && value == "CC Switch")
        );
        assert!(
            product
                .iter()
                .any(|(name, value)| *name == "程序版本" && value == "3.19.2")
        );
        assert!(
            product
                .iter()
                .any(|(name, value)| *name == "程序路径" && value.is_empty())
        );
        let transaction = fields(
            r#"<Event><System><Provider Name="MsiInstaller"/><EventID>1040</EventID></System><EventData><Data>C:\Temp\CC Switch-installer.msi</Data><Data>20252</Data></EventData></Event>"#,
        );
        assert!(transaction.iter().any(|(name, value)| {
            *name == "程序名称" && value == "CC Switch-installer.msi"
        }));
        assert!(transaction.iter().any(|(name, value)| {
            *name == "程序路径" && value == r"C:\Temp\CC Switch-installer.msi"
        }));
        assert!(transaction.iter().any(|(name, value)| {
            *name == "事务标识" && value == r"C:\Temp\CC Switch-installer.msi"
        }));

        let named_transaction = fields(
            r#"<Event><System><Provider Name="MsiInstaller"/><EventID>1042</EventID></System><EventData><Data>XdrAgentCleanerTransaction</Data><Data>17572</Data></EventData></Event>"#,
        );
        assert!(named_transaction.iter().any(|(name, value)| {
            *name == "事务标识" && value == "XdrAgentCleanerTransaction"
        }));
        assert!(
            named_transaction
                .iter()
                .any(|(name, value)| { *name == "程序名称" && value.is_empty() })
        );
        assert!(
            named_transaction
                .iter()
                .any(|(name, value)| { *name == "程序路径" && value.is_empty() })
        );
    }

    #[test]
    fn parses_wer_application_hang_by_original_positions() {
        let event = parsed(
            r#"<Event><System><Provider Name="Windows Error Reporting"/><EventID>1001</EventID></System><EventData><Data></Data><Data>0</Data><Data>AppHangXProcB1</Data><Data>Not available</Data><Data>0</Data><Data>demo.exe</Data><Data>1.2.3.4</Data><Data>stamp</Data><Data>1d07</Data><Data>128</Data><Data>svchost.exe:eventlog</Data><Data>0.0.0.0</Data><Data></Data><Data></Data><Data></Data><Data>files</Data><Data>queue</Data><Data></Data><Data>0</Data><Data>7ed80b73-90b3-11f1-b0a8-782bcb248609</Data><Data>6</Data></EventData></Event>"#,
        );
        assert!(event.include());
        let values = event.fields();
        assert!(values.iter().any(|(name, value)| {
            *name == "程序名称"
                && matches!(value, crate::parser::definition::CellValue::Text(value) if value == "demo.exe")
        }));
        assert!(values.iter().any(|(name, value)| {
            *name == "进程ID"
                && matches!(value, crate::parser::definition::CellValue::Text(value) if value.is_empty())
        }));
    }

    #[test]
    fn maps_wer_module_version_without_treating_it_as_a_path() {
        let values = fields(
            r#"<Event><System><Provider Name="Windows Error Reporting"/><EventID>1001</EventID></System><EventData><Data>bucket</Data><Data>5</Data><Data>APPCRASH</Data><Data>Not available</Data><Data>0</Data><Data>demo.exe</Data><Data>1.2.3.4</Data><Data>app-stamp</Data><Data>ntdll.dll</Data><Data>10.0.26100.3915</Data><Data>module-stamp</Data><Data>c0000005</Data></EventData></Event>"#,
        );

        assert!(
            values.iter().any(|(name, value)| {
                *name == "故障模块版本" && value == "10.0.26100.3915"
            })
        );
        assert!(
            values
                .iter()
                .any(|(name, value)| *name == "故障模块路径" && value.is_empty())
        );
    }

    #[test]
    fn excludes_non_application_wer_and_unrelated_provider() {
        let update = parsed(
            r#"<Event><System><Provider Name="Windows Error Reporting"/><EventID>1001</EventID></System><EventData><Data></Data><Data>0</Data><Data>WindowsUpdateFailure3</Data></EventData></Event>"#,
        );
        let load_perf = parsed(
            r#"<Event><System><Provider Name="Microsoft-Windows-LoadPerf"/><EventID>1000</EventID></System><EventData><Data Name="param1">WmiApRpl</Data></EventData></Event>"#,
        );
        assert!(!update.include());
        assert!(!load_perf.include());
    }
}
