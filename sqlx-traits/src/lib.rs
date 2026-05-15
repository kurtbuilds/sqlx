//! Semantic SQL type traits shared by database drivers and Rust data crates.

#![forbid(unsafe_code)]

use std::error::Error as StdError;

/// A boxed error returned by semantic SQL type conversions.
pub type BoxDynError = Box<dyn StdError + Send + Sync + 'static>;

/// A semantic SQL type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SqlType {
    /// Textual data.
    Text,
    /// A calendar date without a time of day.
    Date,
    /// A time of day without a date.
    Time,
    /// A calendar date and time without an offset or time zone.
    Timestamp,
    /// An instant in time, commonly represented in SQL as `TIMESTAMP WITH TIME ZONE`.
    TimestampTz,
}

/// A semantic SQL value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SqlValue {
    /// SQL `NULL`.
    Null,
    /// Textual data.
    Text(String),
    /// A calendar date without a time of day.
    Date(SqlDate),
    /// A time of day without a date.
    Time(SqlTime),
    /// A calendar date and time without an offset or time zone.
    Timestamp(SqlDateTime),
    /// An instant in time.
    TimestampTz(SqlTimestamp),
}

/// A proleptic Gregorian calendar date.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SqlDate {
    /// The Gregorian year.
    pub year: i32,
    /// The month number in the range `1..=12`.
    pub month: u8,
    /// The day number in the range `1..=31`.
    pub day: u8,
}

impl SqlDate {
    /// Creates a date from its components without validation.
    pub const fn from_ymd_unchecked(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Returns the number of days since `1970-01-01`.
    pub fn days_since_unix_epoch(self) -> i64 {
        let mut year = i64::from(self.year);
        let month = i64::from(self.month);
        let day = i64::from(self.day);

        year -= i64::from(month <= 2);
        let era = div_floor(year, 400);
        let year_of_era = year - era * 400;
        let month_prime = month + if month > 2 { -3 } else { 9 };
        let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

        era * 146_097 + day_of_era - 719_468
    }

    /// Creates a date from a day count relative to `1970-01-01`.
    pub fn from_days_since_unix_epoch(days: i64) -> Option<Self> {
        let days = days.checked_add(719_468)?;
        let era = div_floor(days, 146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let mut year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_prime = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
        let month = month_prime + if month_prime < 10 { 3 } else { -9 };

        year += i64::from(month <= 2);

        Some(Self {
            year: i32::try_from(year).ok()?,
            month: u8::try_from(month).ok()?,
            day: u8::try_from(day).ok()?,
        })
    }
}

/// A time of day with nanosecond precision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SqlTime {
    /// The hour in the range `0..=23`.
    pub hour: u8,
    /// The minute in the range `0..=59`.
    pub minute: u8,
    /// The second in the range `0..=59`.
    pub second: u8,
    /// The fractional nanosecond in the range `0..=999_999_999`.
    pub nanosecond: u32,
}

impl SqlTime {
    /// The number of nanoseconds in a day.
    pub const NANOS_PER_DAY: u64 = 86_400_000_000_000;
    /// The number of microseconds in a day.
    pub const MICROS_PER_DAY: u64 = 86_400_000_000;

    /// Creates a time from its components without validation.
    pub const fn from_hms_nano_unchecked(
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Self {
        Self {
            hour,
            minute,
            second,
            nanosecond,
        }
    }

    /// Returns the nanosecond count since midnight.
    pub fn nanoseconds_since_midnight(self) -> u64 {
        let seconds =
            u64::from(self.hour) * 3_600 + u64::from(self.minute) * 60 + u64::from(self.second);
        seconds * 1_000_000_000 + u64::from(self.nanosecond)
    }

    /// Returns the microsecond count since midnight, truncating sub-microsecond precision.
    pub fn microseconds_since_midnight(self) -> u64 {
        self.nanoseconds_since_midnight() / 1_000
    }

    /// Creates a time from a nanosecond count since midnight.
    pub fn from_nanoseconds_since_midnight(nanos: u64) -> Option<Self> {
        if nanos >= Self::NANOS_PER_DAY {
            return None;
        }

        let seconds = nanos / 1_000_000_000;
        let nanosecond = u32::try_from(nanos % 1_000_000_000).ok()?;
        let hour = seconds / 3_600;
        let minute = (seconds % 3_600) / 60;
        let second = seconds % 60;

        Some(Self {
            hour: u8::try_from(hour).ok()?,
            minute: u8::try_from(minute).ok()?,
            second: u8::try_from(second).ok()?,
            nanosecond,
        })
    }

    /// Creates a time from a microsecond count since midnight.
    pub fn from_microseconds_since_midnight(micros: u64) -> Option<Self> {
        micros
            .checked_mul(1_000)
            .and_then(Self::from_nanoseconds_since_midnight)
    }
}

/// A calendar date and time without an offset or time zone.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SqlDateTime {
    /// The date component.
    pub date: SqlDate,
    /// The time component.
    pub time: SqlTime,
}

impl SqlDateTime {
    /// Creates a datetime from its date and time components.
    pub const fn new(date: SqlDate, time: SqlTime) -> Self {
        Self { date, time }
    }

    /// Returns microseconds since `1970-01-01T00:00:00`, truncating sub-microsecond precision.
    pub fn microseconds_since_unix_epoch(self) -> i128 {
        i128::from(self.date.days_since_unix_epoch()) * i128::from(SqlTime::MICROS_PER_DAY)
            + i128::from(self.time.microseconds_since_midnight())
    }

    /// Creates a datetime from microseconds since `1970-01-01T00:00:00`.
    pub fn from_microseconds_since_unix_epoch(micros: i128) -> Option<Self> {
        let micros_per_day = i128::from(SqlTime::MICROS_PER_DAY);
        let days = micros.div_euclid(micros_per_day);
        let day_micros = micros.rem_euclid(micros_per_day);

        Some(Self {
            date: SqlDate::from_days_since_unix_epoch(i64::try_from(days).ok()?)?,
            time: SqlTime::from_microseconds_since_midnight(u64::try_from(day_micros).ok()?)?,
        })
    }
}

/// An instant in time represented as nanoseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SqlTimestamp {
    unix_nanoseconds: i128,
}

impl SqlTimestamp {
    /// Creates a timestamp from nanoseconds since the Unix epoch.
    pub const fn from_unix_nanoseconds(unix_nanoseconds: i128) -> Self {
        Self { unix_nanoseconds }
    }

    /// Creates a timestamp from microseconds since the Unix epoch.
    pub const fn from_unix_microseconds(unix_microseconds: i64) -> Self {
        Self {
            unix_nanoseconds: unix_microseconds as i128 * 1_000,
        }
    }

    /// Returns nanoseconds since the Unix epoch.
    pub const fn unix_nanoseconds(self) -> i128 {
        self.unix_nanoseconds
    }

    /// Returns microseconds since the Unix epoch, truncating sub-microsecond precision.
    pub fn unix_microseconds_truncated(self) -> Option<i64> {
        i64::try_from(self.unix_nanoseconds / 1_000).ok()
    }
}

/// A type with a known semantic SQL type.
pub trait HasSqlType {
    /// The semantic SQL type this Rust type maps to.
    const SQL_TYPE: SqlType;
}

/// Encodes a Rust value into a semantic SQL value.
pub trait SqlEncode: HasSqlType {
    /// Encodes this value.
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError>;
}

/// Decodes a Rust value from a semantic SQL value.
pub trait SqlDecode: HasSqlType + Sized {
    /// Decodes this value.
    fn decode_sql(value: SqlValue) -> Result<Self, BoxDynError>;
}

impl<T> HasSqlType for &T
where
    T: HasSqlType + ?Sized,
{
    const SQL_TYPE: SqlType = T::SQL_TYPE;
}

impl<T> SqlEncode for &T
where
    T: SqlEncode + ?Sized,
{
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError> {
        (**self).encode_sql()
    }
}

impl<T> HasSqlType for Option<T>
where
    T: HasSqlType,
{
    const SQL_TYPE: SqlType = T::SQL_TYPE;
}

impl<T> SqlEncode for Option<T>
where
    T: SqlEncode,
{
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError> {
        self.as_ref()
            .map_or(Ok(SqlValue::Null), SqlEncode::encode_sql)
    }
}

impl<T> SqlDecode for Option<T>
where
    T: SqlDecode,
{
    fn decode_sql(value: SqlValue) -> Result<Self, BoxDynError> {
        match value {
            SqlValue::Null => Ok(None),
            value => T::decode_sql(value).map(Some),
        }
    }
}

impl HasSqlType for str {
    const SQL_TYPE: SqlType = SqlType::Text;
}

impl SqlEncode for str {
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError> {
        Ok(SqlValue::Text(self.to_owned()))
    }
}

impl HasSqlType for String {
    const SQL_TYPE: SqlType = SqlType::Text;
}

impl SqlEncode for String {
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError> {
        Ok(SqlValue::Text(self.clone()))
    }
}

impl SqlDecode for String {
    fn decode_sql(value: SqlValue) -> Result<Self, BoxDynError> {
        match value {
            SqlValue::Text(text) => Ok(text),
            value => Err(unexpected_sql_value("text", &value)),
        }
    }
}

/// Creates a conversion error for an unexpected semantic SQL value.
pub fn unexpected_sql_value(expected: &'static str, actual: &SqlValue) -> BoxDynError {
    format!("expected {expected}, got {}", actual.kind()).into()
}

impl SqlValue {
    fn kind(&self) -> &'static str {
        match self {
            SqlValue::Null => "null",
            SqlValue::Text(_) => "text",
            SqlValue::Date(_) => "date",
            SqlValue::Time(_) => "time",
            SqlValue::Timestamp(_) => "timestamp",
            SqlValue::TimestampTz(_) => "timestamp with time zone",
        }
    }
}

fn div_floor(n: i64, d: i64) -> i64 {
    let q = n / d;
    let r = n % d;

    if r != 0 && ((r > 0) != (d > 0)) {
        q - 1
    } else {
        q
    }
}
