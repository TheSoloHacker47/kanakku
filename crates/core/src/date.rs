/// A calendar date, stored as days since 1970-01-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(i32);

const IST_OFFSET_MS: i64 = 5 * 3_600_000 + 1_800_000;

impl Date {
    pub fn from_ymd(y: i32, m: u32, d: u32) -> Option<Date> {
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return None;
        }
        // Howard Hinnant's days_from_civil.
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let mp = (m as i32 + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d as i32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Date(era * 146_097 + doe - 719_468))
    }

    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        (y, m, d)
    }

    /// Parses KIIFB's `dd-mm-yyyy`.
    pub fn parse_dmy(s: &str) -> Option<Date> {
        let mut it = s.trim().split('-');
        let d = it.next()?.parse().ok()?;
        let m = it.next()?.parse().ok()?;
        let y = it.next()?.parse().ok()?;
        if it.next().is_some() {
            return None;
        }
        Date::from_ymd(y, m, d)
    }

    /// Parses `yyyy-mm-dd`, ignoring anything after the date part.
    pub fn parse_iso(s: &str) -> Option<Date> {
        let s = s.trim().get(..10)?;
        let mut it = s.split('-');
        let y = it.next()?.parse().ok()?;
        let m = it.next()?.parse().ok()?;
        let d = it.next()?.parse().ok()?;
        Date::from_ymd(y, m, d)
    }

    pub fn to_iso(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }

    /// `dd-mm-yyyy`, the order readers in India expect.
    pub fn to_dmy(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{d:02}-{m:02}-{y:04}")
    }

    /// The date in India for a Unix timestamp in milliseconds.
    pub fn from_unix_ms_ist(ms: i64) -> Date {
        Date((ms + IST_OFFSET_MS).div_euclid(86_400_000) as i32)
    }

    /// The date in India for a UTC timestamp such as `2026-09-30T21:00:05.123Z`.
    pub fn from_utc_timestamp_ist(s: &str) -> Option<Date> {
        let day = Date::parse_iso(s)?;
        let time = s.trim().get(11..16)?;
        let (h, m) = time.split_once(':')?;
        let minutes: i32 = h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?;
        Some(Date(day.0 + (minutes + 330).div_euclid(1440)))
    }

    pub fn days_since(self, earlier: Date) -> i32 {
        self.0 - earlier.0
    }
}

/// Dates travel as `yyyy-mm-dd` strings, in the database and in the open data.
impl serde::Serialize for Date {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_iso())
    }
}

impl<'de> serde::Deserialize<'de> for Date {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<str>>::deserialize(d)?;
        Date::parse_iso(&s).ok_or_else(|| serde::de::Error::custom("expected a yyyy-mm-dd date"))
    }
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2017, 11, 17), (2026, 9, 30), (2100, 12, 31)] {
            let date = Date::from_ymd(y, m, d).unwrap();
            assert_eq!(date.ymd(), (y, m, d));
        }
        assert_eq!(Date::from_ymd(1970, 1, 1).unwrap().0, 0);
    }

    #[test]
    fn parses_kiifb_dates() {
        let d = Date::parse_dmy("17-11-2017").unwrap();
        assert_eq!(d.to_iso(), "2017-11-17");
        assert_eq!(d.to_dmy(), "17-11-2017");
        assert_eq!(Date::parse_iso("2026-01-31T00:00:00Z").unwrap().to_dmy(), "31-01-2026");
    }

    #[test]
    fn utc_timestamps_convert_to_the_india_date() {
        let ist = |s| Date::from_utc_timestamp_ist(s).map(Date::to_iso);
        assert_eq!(ist("2026-09-30T18:29:59.000Z").as_deref(), Some("2026-09-30"));
        assert_eq!(ist("2026-09-30T18:30:00.000Z").as_deref(), Some("2026-10-01"));
        assert_eq!(ist("2026-09-30T21:00:05.123Z").as_deref(), Some("2026-10-01"));
        assert_eq!(ist("2026-09-30"), None);
    }

    #[test]
    fn serialises_as_iso() {
        let d = Date::parse_dmy("31-01-2026").unwrap();
        assert_eq!(serde_json::to_string(&d).unwrap(), "\"2026-01-31\"");
        assert_eq!(serde_json::from_str::<Date>("\"2026-01-31\"").unwrap(), d);
        assert!(serde_json::from_str::<Date>("\"31-01-2026\"").is_err());
    }

    #[test]
    fn rejects_bad_dates() {
        assert!(Date::parse_dmy("31-02-2024").is_none());
        assert!(Date::parse_dmy("29-02-2023").is_none());
        assert!(Date::parse_dmy("1-13-2024").is_none());
        assert!(Date::parse_dmy("").is_none());
        assert!(Date::parse_dmy("2024-01-01-01").is_none());
    }

    #[test]
    fn day_arithmetic() {
        let a = Date::parse_dmy("24-12-2025").unwrap();
        let b = Date::parse_dmy("30-09-2026").unwrap();
        assert_eq!(b.days_since(a), 280);
        assert!(b > a);
    }

    #[test]
    fn india_date_rolls_over_at_1830_utc() {
        // 2026-09-30 18:29:59 UTC is still the 30th in India; one second later it is the 1st.
        let before = 1_790_792_999_000;
        assert_eq!(Date::from_unix_ms_ist(before).to_iso(), "2026-09-30");
        assert_eq!(Date::from_unix_ms_ist(before + 1_000).to_iso(), "2026-10-01");
    }
}
