//! Reading and comparing the dates a conference form collects.
//!
//! A plugin has no clock and no date library: the kernel links no WASI clock
//! into a module, and pulling `chrono` into a WASM plugin to compare two
//! `YYYY-MM-DD` strings would be a dependency for arithmetic. So the arithmetic
//! is here, with tests.
//!
//! Dates are compared as **days**, never as strings. `"2027-10-01"` sorts before
//! `"2027-09-02"` byte by byte and is a month later in fact.

/// Days since 1970-01-01 for a `YYYY-MM-DD` date, or `None` if it is not one.
///
/// Howard Hinnant's `days_from_civil`, proleptic Gregorian. The format is exact:
/// four digits, two, two. `2027-5-1` is rejected rather than guessed at, because
/// the kernel's `Date` fields are written zero-padded and a form that silently
/// accepted both shapes would store both.
pub fn to_days(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (y, m, d) = (
        y.parse::<i64>().ok()?,
        m.parse::<i64>().ok()?,
        d.parse::<i64>().ok()?,
    );
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Whether `date` is a date this form will accept.
pub fn is_valid(date: &str) -> bool {
    to_days(date).is_some()
}

/// Whether `first` falls strictly after `second`. `false` if either is unusable.
pub fn is_after(first: &str, second: &str) -> bool {
    match (to_days(first), to_days(second)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_convert() {
        assert_eq!(to_days("1970-01-01"), Some(0));
        assert_eq!(to_days("2000-03-01"), Some(11_017));
        assert_eq!(to_days("2024-02-29"), Some(19_782));
    }

    #[test]
    fn a_malformed_date_is_not_a_date() {
        for bad in [
            "",
            "soon",
            "2027-13-01",
            "2027-00-01",
            "2027-01-32",
            "2027-5-1",
            "27-05-01",
        ] {
            assert!(to_days(bad).is_none(), "{bad:?} parsed");
            assert!(!is_valid(bad), "{bad:?} is valid");
        }
    }

    #[test]
    fn ordering_is_by_date_not_by_string() {
        // The pair a string comparison gets wrong: October sorts before
        // September byte by byte.
        assert!(is_after("2027-10-01", "2027-09-02"));
        assert!(!is_after("2027-09-02", "2027-10-01"));
    }

    #[test]
    fn the_same_day_is_not_after_itself() {
        assert!(!is_after("2027-05-03", "2027-05-03"));
    }

    #[test]
    fn an_unusable_date_is_never_after_anything() {
        // `false` rather than a panic or a guess: whether one date follows
        // another is unanswerable when one of them is not a date, and the
        // caller has already complained about the format separately.
        assert!(!is_after("soon", "2027-05-03"));
        assert!(!is_after("2027-05-03", "soon"));
    }
}
