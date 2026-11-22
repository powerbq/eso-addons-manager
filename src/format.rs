pub fn compact_number(value: u64) -> String {
    let (scaled, suffix) = match value {
        1_000_000.. => (value as f64 / 1_000_000.0, "M"),
        1_000.. => (value as f64 / 1_000.0, "K"),
        _ => return value.to_string(),
    };

    let formatted = format!("{scaled:.1}");
    let formatted = formatted.strip_suffix(".0").unwrap_or(&formatted);

    format!("{formatted}{suffix}")
}

pub fn date(unix_ms: u64) -> String {
    let days = (unix_ms / 86_400_000) as i64;
    let (year, month, day) = civil_from_days(days);

    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_compact_numbers() {
        assert_eq!(compact_number(0), "0");
        assert_eq!(compact_number(999), "999");
        assert_eq!(compact_number(1_000), "1K");
        assert_eq!(compact_number(24_549), "24.5K");
        assert_eq!(compact_number(6_346_016), "6.3M");
        assert_eq!(compact_number(2_000_000), "2M");
    }

    #[test]
    fn formats_dates_in_utc() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_778_599_485_000), "2026-05-12");
        assert_eq!(date(951_782_400_000), "2000-02-29");
    }
}
