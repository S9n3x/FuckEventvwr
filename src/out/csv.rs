// 数据导出CSV文件
use crate::parser::definition::EventRecord;
use chrono::Local;
use std::fs;

pub fn to_csv(data: &[Box<dyn EventRecord + Send>]) {
    if data.is_empty() {
        return;
    }

    let now = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.csv", data[0].type_name(), now);

    let mut csv = String::from('\u{feff}');
    csv.push_str(&data[0].csv_header());
    csv.push('\n');
    for r in data {
        csv.push_str(&r.to_csv_row());
        csv.push('\n');
    }
    fs::write(&filename, csv).expect("写入 CSV 失败");
}
