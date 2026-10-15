//! Parsing for bulk imports: dictionary CSV (DIC-06) and snippet JSON
//! (SNP-05). Limits and messages follow Wispr Flow's.

use serde::Serialize;

use crate::t;

pub const MAX_IMPORT_BYTES: usize = 3 * 1024 * 1024;
pub const MAX_IMPORT_ENTRIES: usize = 1000;

/// A row that was skipped, with its 1-based line (CSV) or position (JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedRow {
    pub line: usize,
    pub reason: String,
}

/// One CSV row: a word, or `misspelling,correction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvRow {
    Word(String),
    Correction { misspelling: String, term: String },
}

pub fn check_size(text: &str) -> Result<(), String> {
    if text.len() > MAX_IMPORT_BYTES {
        return Err(t!("error.import.tooLarge"));
    }
    Ok(())
}

/// The message for an import of more than [`MAX_IMPORT_ENTRIES`] new rows.
pub fn too_many() -> String {
    t!(
        "error.import.tooMany",
        max = crate::i18n::number(MAX_IMPORT_ENTRIES as i64)
    )
}

/// Splits CSV into rows: no header, one or two non-empty columns, quoted
/// fields with doubled quotes. Rows with more than two non-empty columns or an
/// unterminated quote are skipped; rows of empty fields are ignored.
pub fn parse_dictionary_csv(text: &str) -> Parsed<CsvRow> {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for record in records(text.trim_start_matches('\u{feff}')) {
        let line = record.line;
        let Some(fields) = record.fields else {
            skipped.push(SkippedRow {
                line,
                reason: t!("error.import.unterminatedQuote"),
            });
            continue;
        };
        let values: Vec<String> = fields
            .into_iter()
            .map(|field| field.trim().to_owned())
            .filter(|field| !field.is_empty())
            .collect();
        match values.as_slice() {
            [] => {}
            [word] => rows.push((line, CsvRow::Word(word.clone()))),
            [misspelling, term] => rows.push((
                line,
                CsvRow::Correction {
                    misspelling: misspelling.clone(),
                    term: term.clone(),
                },
            )),
            _ => skipped.push(SkippedRow {
                line,
                reason: t!("error.import.tooManyColumns"),
            }),
        }
    }
    (rows, skipped)
}

struct Record {
    line: usize,
    /// None when the record ends inside a quoted field.
    fields: Option<Vec<String>>,
}

fn records(text: &str) -> Vec<Record> {
    let mut records = Vec::new();
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while chars.peek().is_some() {
        let start_line = line;
        let mut fields = Vec::new();
        let mut field = String::new();
        let mut quoted = false;
        let mut terminated = true;
        loop {
            let Some(c) = chars.next() else {
                terminated = !quoted;
                fields.push(std::mem::take(&mut field));
                break;
            };
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' if quoted => quoted = false,
                '"' if field.trim().is_empty() => {
                    field.clear();
                    quoted = true;
                }
                '\n' if quoted => {
                    line += 1;
                    field.push(c);
                }
                ',' if !quoted => fields.push(std::mem::take(&mut field)),
                '\r' if !quoted => {}
                '\n' => {
                    line += 1;
                    fields.push(std::mem::take(&mut field));
                    break;
                }
                c => field.push(c),
            }
        }
        records.push(Record {
            line: start_line,
            fields: terminated.then_some(fields),
        });
    }
    records
}

/// One snippet from a JSON import: `{"name": trigger, "text": expansion}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonSnippet {
    pub trigger: String,
    pub expansion: String,
}

/// Parsed rows with their position, and the rows skipped.
pub type Parsed<T> = (Vec<(usize, T)>, Vec<SkippedRow>);

/// Parses `[{"name", "text"}]`. Non-string or blank entries are skipped;
/// other fields are ignored.
pub fn parse_snippet_json(text: &str) -> Result<Parsed<JsonSnippet>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| t!("error.import.badFormat"))?;
    let serde_json::Value::Array(items) = value else {
        return Err(t!("error.import.badFormat"));
    };
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let position = index + 1;
        let field = |name: &str| {
            item.get(name)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        };
        match (field("name"), field("text")) {
            (Some(trigger), Some(expansion)) => rows.push((
                position,
                JsonSnippet {
                    trigger: trigger.to_owned(),
                    expansion: expansion.to_owned(),
                },
            )),
            _ => skipped.push(SkippedRow {
                line: position,
                reason: t!("error.import.needsNameAndText"),
            }),
        }
    }
    Ok((rows, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(csv: &str) -> Vec<CsvRow> {
        parse_dictionary_csv(csv)
            .0
            .into_iter()
            .map(|(_, row)| row)
            .collect()
    }

    #[test]
    fn reads_words_and_corrections() {
        assert_eq!(
            words("Shrimp\njira,Jira\r\nword,\n,,\n"),
            [
                CsvRow::Word("Shrimp".into()),
                CsvRow::Correction {
                    misspelling: "jira".into(),
                    term: "Jira".into()
                },
                CsvRow::Word("word".into()),
            ]
        );
    }

    #[test]
    fn handles_quotes_with_commas_and_doubled_quotes() {
        assert_eq!(
            words("\"Smith, John\"\n\"say \"\"hi\"\"\",Hi\n"),
            [
                CsvRow::Word("Smith, John".into()),
                CsvRow::Correction {
                    misspelling: "say \"hi\"".into(),
                    term: "Hi".into()
                },
            ]
        );
    }

    #[test]
    fn skips_wide_rows_and_unterminated_quotes() {
        let (rows, skipped) = parse_dictionary_csv("a,b,c\nok\n\"open,quote\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].line, 1);
        assert_eq!(skipped[0].reason, "More than two columns");
        assert_eq!(skipped[1].line, 3);
    }

    #[test]
    fn rejects_files_over_three_megabytes() {
        assert!(check_size(&"a".repeat(MAX_IMPORT_BYTES + 1)).is_err());
        assert!(check_size("a").is_ok());
    }

    #[test]
    fn reads_snippet_json_and_skips_bad_entries() {
        let (rows, skipped) = parse_snippet_json(
            r#"[{"name":"sig","text":"Best, Alex","id":7},{"name":"x"},{"name":3,"text":"y"},{"name":"  ","text":"z"}]"#,
        )
        .unwrap();
        assert_eq!(
            rows,
            [(
                1,
                JsonSnippet {
                    trigger: "sig".into(),
                    expansion: "Best, Alex".into()
                }
            )]
        );
        assert_eq!(skipped.len(), 3);
        assert!(parse_snippet_json("{\"name\":\"a\"}").is_err());
        assert!(parse_snippet_json("nope").is_err());
    }
}
