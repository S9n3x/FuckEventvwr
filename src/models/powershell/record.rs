use super::structs::{POWER_SHELL_META, PowerShellDetail};
use crate::parser::definition::{CellValue, EventRecord};

impl EventRecord for PowerShellDetail {
    fn time(&self) -> &str {
        &self.time
    }

    fn type_name(&self) -> &'static str {
        "PowerShell"
    }
    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)> {
        let m = &POWER_SHELL_META;
        vec![
            (m.time.title, CellValue::text(&self.time)),
            (m.event_id.title, CellValue::num(self.event_id)),
            (m.description.title, CellValue::text(self.description)),
            (m.script_block.title, CellValue::text(&self.script_block)),
            (m.command_line.title, CellValue::text(&self.command_line)),
            (m.raw_data.title, CellValue::text(&self.raw_data)),
        ]
    }
}
