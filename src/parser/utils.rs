use std::collections::HashSet;
use std::path::PathBuf;

// 获取EventID
pub fn parse_event_id(xml: &str) -> Option<u16> {
    let system_end = xml.find("</System>")?;
    let system = &xml[..system_end];

    let tag_start = system.find("<EventID")?;
    let value_start = tag_start + system[tag_start..].find('>')? + 1;
    let value_end = value_start + system[value_start..].find("</EventID>")?;

    system[value_start..value_end].trim().parse::<u16>().ok()
}

// 过滤文件名
pub fn filter_event_by_name(all_paths: Vec<PathBuf>, allowed_names: &[&str]) -> Vec<PathBuf> {
    let name_set: HashSet<&str> = allowed_names.iter().cloned().collect();

    all_paths
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |name| name_set.contains(name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_event_id;

    #[test]
    fn parses_plain_event_id() {
        let xml = "<Event><System><EventID>4624</EventID></System></Event>";
        assert_eq!(parse_event_id(xml), Some(4624));
    }

    #[test]
    fn parses_event_id_with_qualifiers() {
        let xml = r#"<Event><System><EventID Qualifiers="16384">11707</EventID></System></Event>"#;
        assert_eq!(parse_event_id(xml), Some(11707));
    }

    #[test]
    fn ignores_event_id_text_outside_system() {
        let xml = concat!(
            "<Event><System><EventID Qualifiers=\"0\">4104</EventID></System>",
            "<EventData><Data Name=\"ScriptBlockText\">",
            "&lt;EventID&gt;9999&lt;/EventID&gt;",
            "</Data></EventData></Event>"
        );
        assert_eq!(parse_event_id(xml), Some(4104));
    }

    #[test]
    fn rejects_missing_or_invalid_event_id() {
        assert_eq!(parse_event_id("<Event><System></System></Event>"), None);
        assert_eq!(
            parse_event_id("<Event><System><EventID>invalid</EventID></System></Event>"),
            None
        );
    }
}
