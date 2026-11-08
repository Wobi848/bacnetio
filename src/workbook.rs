//! The exported lists as an Excel file (docs/concept-excel.md): the same [`Table`]s as the CSV,
//! with the header in bold, frozen and filtered, widths from the entries, numbers as numbers and
//! times as Excel times, so they work in any region.

use std::collections::HashMap;

use rust_xlsxwriter::{ExcelDateTime, Format, Workbook, Worksheet, XlsxError};

use crate::export::Table;

/// Excel's limit for a sheet's name.
const LONGEST_SHEET_NAME: usize = 31;
/// Characters Excel does not allow in a sheet's name.
const NOT_IN_SHEET_NAMES: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];
/// A column is as wide as its longest entry, up to this.
const WIDEST_COLUMN: usize = 60;
const NARROWEST_COLUMN: usize = 6;
const TIME_FORMAT: &str = "yyyy-mm-dd hh:mm:ss";

/// What the page gives in the reader's language: the column heads by their English names, and a
/// line above the header naming the site, device, time and version. Heads not given stay English.
#[derive(Default)]
pub struct Labels {
    pub heads: HashMap<String, String>,
    pub title: Option<String>,
}

/// One workbook of sheets, each `(name, table)`, as the bytes of an `.xlsx` file.
pub fn xlsx(sheets: &[(String, &Table)], labels: &Labels) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let mut used = Vec::new();
    for (name, table) in sheets {
        let name = sheet_name(name, &used);
        used.push(name.clone());
        let sheet = workbook.add_worksheet();
        sheet.set_name(&name)?;
        write_sheet(sheet, table, labels)?;
    }
    workbook.save_to_buffer()
}

fn write_sheet(sheet: &mut Worksheet, table: &Table, labels: &Labels) -> Result<(), XlsxError> {
    let bold = Format::new().set_bold();
    let time = Format::new().set_num_format(TIME_FORMAT);
    let mut header_row = 0;
    if let Some(title) = &labels.title {
        sheet.write_string(0, 0, title)?;
        header_row = 2;
    }
    let heads: Vec<&str> = table
        .columns
        .iter()
        .map(|head| labels.heads.get(head).map_or(head.as_str(), String::as_str))
        .collect();
    let mut widths: Vec<usize> = heads.iter().map(|head| head.chars().count()).collect();
    for (column, head) in heads.iter().enumerate() {
        sheet.write_string_with_format(header_row, column as u16, *head, &bold)?;
    }
    for (index, fields) in table.rows.iter().enumerate() {
        let row = header_row + 1 + index as u32;
        for (column, text) in fields.iter().enumerate() {
            let column_number = column as u16;
            if let Some(number) = as_number(text) {
                sheet.write_number(row, column_number, number)?;
            } else if let Some(moment) = as_time(text) {
                sheet.write_datetime_with_format(row, column_number, &moment, &time)?;
            } else if !text.is_empty() {
                sheet.write_string(row, column_number, text)?;
            }
            if let Some(width) = widths.get_mut(column) {
                *width = (*width).max(text.chars().count());
            }
        }
    }
    for (column, width) in widths.iter().enumerate() {
        let width = (*width).clamp(NARROWEST_COLUMN, WIDEST_COLUMN) + 2;
        sheet.set_column_width(column as u16, width as f64)?;
    }
    if !heads.is_empty() {
        let last_row = header_row + table.rows.len() as u32;
        sheet.autofilter(header_row, 0, last_row, heads.len() as u16 - 1)?;
        sheet.set_freeze_panes(header_row + 1, 0)?;
    }
    Ok(())
}

/// A plain number as the lists write it ("21.4", "-3", "1001"); a number with leading zeros, such
/// as a MAC "0012", stays text.
fn as_number(text: &str) -> Option<f64> {
    let leading_zero = text.len() > 1 && text.starts_with('0') && !text.starts_with("0.");
    if leading_zero || text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

/// A time as the lists write it: "2026-11-01 09:30:00".
fn as_time(text: &str) -> Option<ExcelDateTime> {
    let bytes = text.as_bytes();
    let shaped = bytes.len() == 19 && bytes[4] == b'-' && bytes[10] == b' ' && bytes[13] == b':';
    shaped
        .then(|| ExcelDateTime::parse_from_str(text).ok())
        .flatten()
}

/// A device's name as a sheet name: without the characters Excel refuses, at most 31 characters,
/// and unique in the workbook.
fn sheet_name(name: &str, used: &[String]) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if NOT_IN_SHEET_NAMES.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim_matches('\'').trim();
    let base: String = if clean.is_empty() { "Sheet" } else { clean }
        .chars()
        .take(LONGEST_SHEET_NAME)
        .collect();
    // Excel takes "Lüftung" and "LÜFTUNG" for the same sheet, not only ASCII letters
    let taken = |candidate: &str| {
        let candidate = candidate.to_lowercase();
        used.iter().any(|name| name.to_lowercase() == candidate)
    };
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|number| {
            let suffix = format!(" ({number})");
            let room = LONGEST_SHEET_NAME - suffix.chars().count();
            format!("{}{suffix}", base.chars().take(room).collect::<String>())
        })
        .find(|candidate| !taken(candidate))
        .expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_times_are_recognised_but_codes_stay_text() {
        assert_eq!(as_number("21.4"), Some(21.4));
        assert_eq!(as_number("-3"), Some(-3.0));
        assert_eq!(as_number("0.5"), Some(0.5));
        assert_eq!(as_number("0012"), None, "a MAC keeps its zeros");
        assert_eq!(as_number("active"), None);
        assert_eq!(as_number("NaN"), None);
        assert!(as_time("2026-11-01 09:30:00").is_some());
        assert!(as_time("2026-11-01").is_none());
        assert!(as_time("21.4").is_none());
    }

    #[test]
    fn sheet_names_are_cut_cleaned_and_unique() {
        let long = "LU_Lueftungsanlage_Bueroflaeche_Nord";
        let first = sheet_name(long, &[]);
        assert_eq!(first.chars().count(), 31);
        let second = sheet_name(long, std::slice::from_ref(&first));
        assert!(second.ends_with(" (2)") && second.chars().count() <= 31);
        assert_eq!(sheet_name("DDC4000/#21", &[]), "DDC4000_#21");
        assert_eq!(sheet_name("", &[]), "Sheet");
        assert_eq!(sheet_name("LÜFTUNG", &["Lüftung".to_string()]), "LÜFTUNG (2)");
    }

    /// A part of the file, as XML text.
    fn part(bytes: &[u8], name: &str) -> String {
        use std::io::Read;
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        text
    }

    #[test]
    fn a_workbook_holds_its_sheets_heads_numbers_and_times() {
        let devices = Table {
            columns: vec!["Device ID".into(), "Name".into(), "MAC".into()],
            rows: vec![vec!["1001".into(), "AHU-01".into(), "0012".into()]],
        };
        let log = Table {
            columns: vec!["Time".into()],
            rows: vec![vec!["2026-11-01 09:30:00".into()]],
        };
        let labels = Labels {
            heads: HashMap::from([("Device ID".to_string(), "Geräte-ID".to_string())]),
            title: Some("Sample School · bacnetio 0.3.0".into()),
        };
        let bytes = xlsx(
            &[("Geräte".into(), &devices), ("Log".into(), &log)],
            &labels,
        )
        .unwrap();
        let book = part(&bytes, "xl/workbook.xml");
        assert!(book.contains(r#"name="Geräte""#) && book.contains(r#"name="Log""#));
        let strings = part(&bytes, "xl/sharedStrings.xml");
        assert!(
            strings.contains("Geräte-ID"),
            "the head in the reader's language"
        );
        assert!(strings.contains("Name"), "a head not given stays English");
        assert!(strings.contains("Sample School"), "the title line");
        assert!(strings.contains("0012"), "a MAC stays text");
        let sheet = part(&bytes, "xl/worksheets/sheet1.xml");
        assert!(
            sheet.contains(r#"<c r="A4"><v>1001</v></c>"#),
            "the ID as a number: {sheet}"
        );
        assert!(sheet.contains(r#"ySplit="3""#), "the header stays in view");
        assert!(
            sheet.contains(r#"<autoFilter ref="A3:C4"/>"#),
            "with Excel's filter"
        );
        let times = part(&bytes, "xl/worksheets/sheet2.xml");
        assert!(
            times.contains("<v>46327.395833"),
            "the time as an Excel time: {times}"
        );
    }
}
