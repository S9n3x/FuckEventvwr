use super::structs::{APPLICATION_META, ApplicationDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for ApplicationDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "Application"
    }

    fn include(&self) -> bool {
        self.include
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &APPLICATION_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.provider.title, CellValue::text(&self.provider)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.event_type.title, CellValue::text(&self.event_type)),
            (m.report_type.title, CellValue::text(&self.report_type)),
            (
                m.application_name.title,
                CellValue::text(&self.application_name),
            ),
            (
                m.application_version.title,
                CellValue::text(&self.application_version),
            ),
            (
                m.transaction_identifier.title,
                CellValue::text(&self.transaction_identifier),
            ),
            (
                m.application_path.title,
                CellValue::text(&self.application_path),
            ),
            (m.fault_module.title, CellValue::text(&self.fault_module)),
            (
                m.fault_module_version.title,
                CellValue::text(&self.fault_module_version),
            ),
            (
                m.fault_module_path.title,
                CellValue::text(&self.fault_module_path),
            ),
            (
                m.exception_code.title,
                CellValue::text(&self.exception_code),
            ),
            (m.process_id.title, CellValue::text(&self.process_id)),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
