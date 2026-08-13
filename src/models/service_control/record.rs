use super::structs::{SERVICE_CONTROL_META, ServiceControlDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for ServiceControlDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "ServiceControl"
    }

    fn include(&self) -> bool {
        self.include
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &SERVICE_CONTROL_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.provider.title, CellValue::text(&self.provider)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.service_name.title, CellValue::text(&self.service_name)),
            (
                m.internal_service_name.title,
                CellValue::text(&self.internal_service_name),
            ),
            (
                m.old_start_type.title,
                CellValue::text(&self.old_start_type),
            ),
            (
                m.new_start_type.title,
                CellValue::text(&self.new_start_type),
            ),
            (m.service_type.title, CellValue::text(&self.service_type)),
            (m.start_type.title, CellValue::text(&self.start_type)),
            (m.image_path.title, CellValue::text(&self.image_path)),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
