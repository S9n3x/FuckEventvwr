use super::structs::{SESSION_META, SessionDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for SessionDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "Session"
    }

    fn include(&self) -> bool {
        self.include
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &SESSION_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.provider.title, CellValue::text(&self.provider)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.user_name.title, CellValue::text(&self.user_name)),
            (m.domain_name.title, CellValue::text(&self.domain_name)),
            (m.session_id.title, CellValue::text(&self.session_id)),
            (
                m.logon_type.title,
                match self.logon_type {
                    Some(value) => CellValue::num(value),
                    None => CellValue::text(""),
                },
            ),
            (
                m.remote_address.title,
                CellValue::text(&self.remote_address),
            ),
            (m.reason.title, CellValue::text(&self.reason)),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
