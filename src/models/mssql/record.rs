use super::structs::{MSSQL_META, MssqlDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for MssqlDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "MSSQL"
    }

    fn include(&self) -> bool {
        self.include
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &MSSQL_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.provider.title, CellValue::text(&self.provider)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.event_type.title, CellValue::text(self.event_type)),
            (
                m.database_instance.title,
                CellValue::text(&self.database_instance),
            ),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
