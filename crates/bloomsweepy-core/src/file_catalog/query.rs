use super::FileCatalogEntryKind;

const MAX_QUERY_TOKENS: usize = 32;
const MAX_QUERY_EXTENSIONS: usize = 32;
const MILLIS_PER_DAY: u64 = 86_400_000;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ParsedCatalogQuery {
    pub terms: Vec<String>,
    pub excluded_terms: Vec<String>,
    pub path_terms: Vec<String>,
    pub excluded_path_terms: Vec<String>,
    pub extensions: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub kind: Option<FileCatalogEntryKind>,
    pub excluded_kinds: Vec<FileCatalogEntryKind>,
    pub min_bytes: Option<u64>,
    pub max_bytes: Option<u64>,
    pub modified_after_ms: Option<u64>,
    pub modified_before_ms: Option<u64>,
}

impl ParsedCatalogQuery {
    fn has_positive_selector(&self) -> bool {
        !self.terms.is_empty()
            || !self.path_terms.is_empty()
            || !self.extensions.is_empty()
            || self.kind.is_some()
            || self.min_bytes.is_some()
            || self.max_bytes.is_some()
            || self.modified_after_ms.is_some()
            || self.modified_before_ms.is_some()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct QueryToken {
    value: String,
    excluded: bool,
}

pub(super) fn parse_catalog_query(
    query: &str,
    timezone_offset_minutes: i32,
) -> Result<ParsedCatalogQuery, String> {
    if !(-1_440..=1_440).contains(&timezone_offset_minutes) {
        return Err("timezone offset is outside the supported range".to_owned());
    }
    let mut parsed = ParsedCatalogQuery::default();
    for token in tokenize(query)? {
        let Some((prefix, value)) = token.value.split_once(':') else {
            push_text_term(&mut parsed, token);
            continue;
        };
        let prefix = prefix.to_ascii_lowercase();
        match prefix.as_str() {
            "ext" | "extension" => {
                let extensions = parse_extensions(value)?;
                let destination = if token.excluded {
                    &mut parsed.excluded_extensions
                } else {
                    &mut parsed.extensions
                };
                destination.extend(extensions);
            }
            "type" | "kind" => {
                let kind = parse_kind(value)?;
                if token.excluded {
                    parsed.excluded_kinds.push(kind);
                } else if parsed.kind.is_some_and(|current| current != kind) {
                    return Err("type filters select different entry kinds".to_owned());
                } else {
                    parsed.kind = Some(kind);
                }
            }
            "path" | "in" => {
                let value = non_empty_value("path", value)?;
                if token.excluded {
                    parsed.excluded_path_terms.push(value.to_owned());
                } else {
                    parsed.path_terms.push(value.to_owned());
                }
            }
            "size" => {
                if token.excluded {
                    return Err("size filters cannot be negated".to_owned());
                }
                apply_size_constraint(&mut parsed, value)?;
            }
            "after" => {
                if token.excluded {
                    return Err("after filters cannot be negated".to_owned());
                }
                let value =
                    parse_date_start_ms(non_empty_value("after", value)?, timezone_offset_minutes)?;
                parsed.modified_after_ms = Some(
                    parsed
                        .modified_after_ms
                        .map_or(value, |current| current.max(value)),
                );
            }
            "before" => {
                if token.excluded {
                    return Err("before filters cannot be negated".to_owned());
                }
                let value = parse_date_start_ms(
                    non_empty_value("before", value)?,
                    timezone_offset_minutes,
                )?;
                parsed.modified_before_ms = Some(
                    parsed
                        .modified_before_ms
                        .map_or(value, |current| current.min(value)),
                );
            }
            _ => push_text_term(&mut parsed, token),
        }
    }

    sort_and_deduplicate(&mut parsed.extensions);
    sort_and_deduplicate(&mut parsed.excluded_extensions);
    if parsed.extensions.len() > MAX_QUERY_EXTENSIONS
        || parsed.excluded_extensions.len() > MAX_QUERY_EXTENSIONS
    {
        return Err(format!(
            "use at most {MAX_QUERY_EXTENSIONS} included and excluded extensions"
        ));
    }
    parsed
        .excluded_kinds
        .sort_by_key(|kind| kind.database_value());
    parsed.excluded_kinds.dedup();

    if parsed
        .min_bytes
        .zip(parsed.max_bytes)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err("size filters do not overlap".to_owned());
    }
    if parsed
        .modified_after_ms
        .zip(parsed.modified_before_ms)
        .is_some_and(|(after, before)| after >= before)
    {
        return Err("date filters do not overlap".to_owned());
    }
    if !parsed.has_positive_selector() {
        return Err("add a word or a positive structured filter before exclusions".to_owned());
    }

    Ok(parsed)
}

fn tokenize(query: &str) -> Result<Vec<QueryToken>, String> {
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut quoted = false;
    let mut characters = query.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '"' => quoted = !quoted,
            '\\' if quoted && characters.peek() == Some(&'"') => {
                characters.next();
                buffer.push('"');
            }
            character if character.is_whitespace() && !quoted => {
                push_token(&mut tokens, &mut buffer)?;
            }
            _ => buffer.push(character),
        }
    }

    if quoted {
        return Err("close the quoted search phrase".to_owned());
    }
    push_token(&mut tokens, &mut buffer)?;
    Ok(tokens)
}

fn push_token(tokens: &mut Vec<QueryToken>, buffer: &mut String) -> Result<(), String> {
    if buffer.is_empty() {
        return Ok(());
    }
    if tokens.len() >= MAX_QUERY_TOKENS {
        return Err(format!("use at most {MAX_QUERY_TOKENS} search terms"));
    }
    let value = std::mem::take(buffer);
    let (excluded, value) = value
        .strip_prefix('-')
        .filter(|value| !value.is_empty())
        .map_or((false, value.as_str()), |value| (true, value));
    tokens.push(QueryToken {
        value: value.to_owned(),
        excluded,
    });
    Ok(())
}

fn push_text_term(parsed: &mut ParsedCatalogQuery, token: QueryToken) {
    if token.excluded {
        parsed.excluded_terms.push(token.value);
    } else {
        parsed.terms.push(token.value);
    }
}

fn non_empty_value<'a>(name: &str, value: &'a str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{name} filter needs a value"))
    } else {
        Ok(value)
    }
}

fn parse_extensions(value: &str) -> Result<Vec<String>, String> {
    let extensions = value
        .split([',', ';'])
        .map(|extension| {
            extension
                .trim()
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|extension| !extension.is_empty())
        .collect::<Vec<_>>();
    if extensions.is_empty() {
        return Err("ext filter needs at least one extension".to_owned());
    }
    if extensions.iter().any(|extension| {
        extension.len() > 32
            || !extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    }) {
        return Err("extensions may contain only letters and numbers".to_owned());
    }
    Ok(extensions)
}

fn parse_kind(value: &str) -> Result<FileCatalogEntryKind, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "file" => Ok(FileCatalogEntryKind::File),
        "dir" | "directory" | "folder" => Ok(FileCatalogEntryKind::Directory),
        "link" | "symlink" => Ok(FileCatalogEntryKind::Symlink),
        "other" => Ok(FileCatalogEntryKind::Other),
        _ => Err("type must be file, folder, symlink, or other".to_owned()),
    }
}

fn apply_size_constraint(parsed: &mut ParsedCatalogQuery, value: &str) -> Result<(), String> {
    let compact = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if let Some((minimum, maximum)) = compact.split_once("..") {
        let minimum = parse_size_value(non_empty_value("size range", minimum)?)?;
        let maximum = parse_size_value(non_empty_value("size range", maximum)?)?;
        tighten_minimum(&mut parsed.min_bytes, minimum);
        tighten_maximum(&mut parsed.max_bytes, maximum);
        return Ok(());
    }

    let (operator, amount) = [">=", "<=", ">", "<", "="]
        .into_iter()
        .find_map(|operator| {
            compact
                .strip_prefix(operator)
                .map(|amount| (operator, amount))
        })
        .ok_or_else(|| "size needs >, >=, <, <=, =, or a range such as 10mb..1gb".to_owned())?;
    let bytes = parse_size_value(non_empty_value("size", amount)?)?;
    match operator {
        ">=" => tighten_minimum(&mut parsed.min_bytes, bytes),
        ">" => tighten_minimum(
            &mut parsed.min_bytes,
            bytes
                .checked_add(1)
                .ok_or_else(|| "size is above the supported range".to_owned())?,
        ),
        "<=" => tighten_maximum(&mut parsed.max_bytes, bytes),
        "<" => tighten_maximum(
            &mut parsed.max_bytes,
            bytes
                .checked_sub(1)
                .ok_or_else(|| "size must be greater than zero".to_owned())?,
        ),
        "=" => {
            tighten_minimum(&mut parsed.min_bytes, bytes);
            tighten_maximum(&mut parsed.max_bytes, bytes);
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn parse_size_value(value: &str) -> Result<u64, String> {
    let split_at = value
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_digit() && character != '.').then_some(index)
        })
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split_at);
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1_u128,
        "k" | "kb" | "kib" => 1_024,
        "m" | "mb" | "mib" => 1_048_576,
        "g" | "gb" | "gib" => 1_073_741_824,
        "t" | "tb" | "tib" => 1_099_511_627_776,
        _ => return Err("size unit must be B, KB, MB, GB, or TB".to_owned()),
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
        || fraction.len() > 6
        || number.matches('.').count() > 1
    {
        return Err("size must be a positive number with at most six decimal places".to_owned());
    }
    let whole = whole
        .parse::<u128>()
        .map_err(|_| "size is above the supported range".to_owned())?;
    let mut bytes = whole
        .checked_mul(multiplier)
        .ok_or_else(|| "size is above the supported range".to_owned())?;
    if !fraction.is_empty() {
        let fraction_value = fraction
            .parse::<u128>()
            .map_err(|_| "size is not valid".to_owned())?;
        let scale = 10_u128.pow(fraction.len() as u32);
        bytes = bytes
            .checked_add(fraction_value.saturating_mul(multiplier) / scale)
            .ok_or_else(|| "size is above the supported range".to_owned())?;
    }
    u64::try_from(bytes).map_err(|_| "size is above the supported range".to_owned())
}

fn tighten_minimum(current: &mut Option<u64>, value: u64) {
    *current = Some(current.map_or(value, |current| current.max(value)));
}

fn tighten_maximum(current: &mut Option<u64>, value: u64) {
    *current = Some(current.map_or(value, |current| current.min(value)));
}

fn parse_date_start_ms(value: &str, timezone_offset_minutes: i32) -> Result<u64, String> {
    let mut parts = value.split('-');
    let year = parts.next().and_then(|value| value.parse::<i64>().ok());
    let month = parts.next().and_then(|value| value.parse::<u32>().ok());
    let day = parts.next().and_then(|value| value.parse::<u32>().ok());
    if parts.next().is_some() {
        return Err("dates must use YYYY-MM-DD".to_owned());
    }
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return Err("dates must use YYYY-MM-DD".to_owned());
    };
    if !(1970..=9999).contains(&year) || !(1..=12).contains(&month) {
        return Err("date is outside the supported range".to_owned());
    }
    let maximum_day = days_in_month(year, month);
    if day == 0 || day > maximum_day {
        return Err("date is not a valid calendar day".to_owned());
    }
    let utc_millis = i128::from(days_from_civil(year, month, day))
        .checked_mul(i128::from(MILLIS_PER_DAY))
        .and_then(|millis| millis.checked_add(i128::from(timezone_offset_minutes) * 60_000))
        .ok_or_else(|| "date is outside the supported range".to_owned())?;
    u64::try_from(utc_millis).map_err(|_| "date is outside the supported range".to_owned())
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn sort_and_deduplicate(values: &mut Vec<String>) {
    values.sort_unstable();
    values.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_filters_quotes_and_exclusions() {
        let parsed = parse_catalog_query(
            r#""annual report" -draft path:"Team Files" -path:archive ext:.PDF,jpg -ext:tmp type:file -type:symlink"#,
            0,
        )
        .expect("parse query");

        assert_eq!(parsed.terms, ["annual report"]);
        assert_eq!(parsed.excluded_terms, ["draft"]);
        assert_eq!(parsed.path_terms, ["Team Files"]);
        assert_eq!(parsed.excluded_path_terms, ["archive"]);
        assert_eq!(parsed.extensions, ["jpg", "pdf"]);
        assert_eq!(parsed.excluded_extensions, ["tmp"]);
        assert_eq!(parsed.kind, Some(FileCatalogEntryKind::File));
        assert_eq!(parsed.excluded_kinds, [FileCatalogEntryKind::Symlink]);
    }

    #[test]
    fn parses_size_ranges_and_tightens_constraints() {
        let parsed = parse_catalog_query("size:>=1.5kb size:<2kb", 0).expect("parse sizes");
        assert_eq!(parsed.min_bytes, Some(1_536));
        assert_eq!(parsed.max_bytes, Some(2_047));

        let range = parse_catalog_query("size:10mb..1gb", 0).expect("parse range");
        assert_eq!(range.min_bytes, Some(10 * 1_048_576));
        assert_eq!(range.max_bytes, Some(1_073_741_824));
    }

    #[test]
    fn parses_calendar_bounds_in_the_requested_timezone() {
        let parsed =
            parse_catalog_query("after:2026-01-01 before:2026-01-03", 0).expect("parse dates");
        assert_eq!(
            parsed.modified_before_ms.unwrap() - parsed.modified_after_ms.unwrap(),
            2 * MILLIS_PER_DAY
        );
        assert_eq!(parse_date_start_ms("1970-01-01", 0).unwrap(), 0);
        assert_eq!(parse_date_start_ms("1970-01-02", -540).unwrap(), 54_000_000);
        assert!(parse_date_start_ms("2025-02-29", 0).is_err());
        assert!(parse_date_start_ms("2024-02-29", 0).is_ok());
    }

    #[test]
    fn rejects_ambiguous_or_malformed_filters() {
        assert!(parse_catalog_query("type:file type:folder", 0).is_err());
        assert!(parse_catalog_query("size:large", 0).is_err());
        assert!(parse_catalog_query("after:2026-13-01", 0).is_err());
        assert!(parse_catalog_query("\"unfinished", 0).is_err());
        assert!(parse_catalog_query("\"\"", 0).is_err());
        assert!(parse_catalog_query("-draft -ext:tmp", 0).is_err());
        assert!(parse_catalog_query("after:2026-01-01", 1_441).is_err());
    }
}
