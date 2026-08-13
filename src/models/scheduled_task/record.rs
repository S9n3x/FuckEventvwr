use super::structs::{SCHEDULED_TASK_META, ScheduledTaskDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for ScheduledTaskDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "ScheduledTask"
    }

    fn include(&self) -> bool {
        self.include
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &SCHEDULED_TASK_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.task_name.title, CellValue::text(&self.task_name)),
            (
                m.subject_user_name.title,
                CellValue::text(&self.subject_user_name),
            ),
            (m.action.title, CellValue::text(&self.action)),
            (m.result_code.title, CellValue::text(&self.result_code)),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
