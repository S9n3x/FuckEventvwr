// use crate::out::tui::run_tui;
use super::csv::to_csv;
use crate::cfg::sturct::OutFormat;

pub fn run(
    mut data: Vec<Box<dyn crate::parser::definition::EventRecord + Send>>,
    of: OutFormat,
) -> usize {
    data.retain(|record| record.include());
    let matched = data.len();
    // 数据排序
    data.sort_by(|a, b| b.time().cmp(a.time()));
    match of {
        OutFormat::Csv => to_csv(&data),
        OutFormat::Excel => crate::out::excel::to_excel(&data),
    }
    matched
}
