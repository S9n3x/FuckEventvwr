use std::borrow::Cow;

pub struct FieldMeta {
    pub title: &'static str,
    // TODO: 后续可以增加字段宽度
}

pub enum CellValue<'a> {
    Text(Cow<'a, str>),
    Number(f64),
}

impl<'a> CellValue<'a> {
    pub fn text(s: &'a str) -> Self {
        CellValue::Text(Cow::Borrowed(s))
    }

    pub fn num(n: impl Into<f64>) -> Self {
        CellValue::Number(n.into())
    }

    /// 转为 CSV 字段字符串（数值无需引号转义）
    fn to_csv_string(&self) -> String {
        match self {
            CellValue::Text(cow) => {
                let s = cow.as_ref();
                if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            }
            CellValue::Number(n) => n.to_string(),
        }
    }
}

pub trait EventRecord {
    // general
    fn time(&self) -> &str;
    fn type_name(&self) -> &'static str;

    fn include(&self) -> bool {
        true
    }

    fn fields(&self) -> Vec<(&'static str, CellValue<'_>)>;

    fn csv_header(&self) -> String {
        self.fields()
            .iter()
            .map(|(t, _)| *t)
            .collect::<Vec<_>>()
            .join(",")
    }
    fn to_csv_row(&self) -> String {
        self.fields()
            .iter()
            .map(|(_, v)| v.to_csv_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}
