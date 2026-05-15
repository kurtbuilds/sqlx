use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::types::Type;
use crate::{PgArgumentBuffer, PgTypeInfo, PgValueFormat, PgValueRef, Postgres};
use sqlx_core::sql_traits::{
    DatabaseSqlTypes, SqlDate, SqlDateTime, SqlTime, SqlTimestamp, SqlType, SqlValue,
};
use sqlx_core::type_info::TypeInfo;

const POSTGRES_EPOCH_DAYS_FROM_UNIX_EPOCH: i64 = 10_957;
const POSTGRES_EPOCH_MICROS_FROM_UNIX_EPOCH: i128 = 946_684_800_000_000;

impl DatabaseSqlTypes for Postgres {
    fn sql_type_info(sql_type: SqlType) -> PgTypeInfo {
        match sql_type {
            SqlType::Text => PgTypeInfo::TEXT,
            SqlType::Date => PgTypeInfo::DATE,
            SqlType::Time => PgTypeInfo::TIME,
            SqlType::Timestamp => PgTypeInfo::TIMESTAMP,
            SqlType::TimestampTz => PgTypeInfo::TIMESTAMPTZ,
        }
    }

    fn sql_type_compatible(sql_type: SqlType, type_info: &PgTypeInfo) -> bool {
        match sql_type {
            SqlType::Text => <str as Type<Postgres>>::compatible(type_info),
            SqlType::Date => PgTypeInfo::DATE.type_compatible(type_info),
            SqlType::Time => PgTypeInfo::TIME.type_compatible(type_info),
            SqlType::Timestamp => PgTypeInfo::TIMESTAMP.type_compatible(type_info),
            SqlType::TimestampTz => PgTypeInfo::TIMESTAMPTZ.type_compatible(type_info),
        }
    }

    fn encode_sql_value(
        value: SqlValue,
        buf: &mut PgArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        match value {
            SqlValue::Null => Ok(IsNull::Yes),
            SqlValue::Text(text) => Encode::<Postgres>::encode(text, buf),
            SqlValue::Date(date) => {
                let days = date.days_since_unix_epoch() - POSTGRES_EPOCH_DAYS_FROM_UNIX_EPOCH;
                let days = i32::try_from(days)?;
                Encode::<Postgres>::encode(days, buf)
            }
            SqlValue::Time(time) => {
                let micros = i64::try_from(time.microseconds_since_midnight())?;
                Encode::<Postgres>::encode(micros, buf)
            }
            SqlValue::Timestamp(datetime) => {
                let micros = datetime.microseconds_since_unix_epoch()
                    - POSTGRES_EPOCH_MICROS_FROM_UNIX_EPOCH;
                let micros = i64::try_from(micros)?;
                Encode::<Postgres>::encode(micros, buf)
            }
            SqlValue::TimestampTz(timestamp) => {
                let unix_micros = timestamp
                    .unix_microseconds_truncated()
                    .ok_or("timestamp is out of range for PostgreSQL")?;
                let micros = i128::from(unix_micros) - POSTGRES_EPOCH_MICROS_FROM_UNIX_EPOCH;
                let micros = i64::try_from(micros)?;
                Encode::<Postgres>::encode(micros, buf)
            }
        }
    }

    fn decode_sql_value(sql_type: SqlType, value: PgValueRef<'_>) -> Result<SqlValue, BoxDynError> {
        match value.format() {
            PgValueFormat::Text => return Ok(SqlValue::Text(value.as_str()?.to_owned())),
            PgValueFormat::Binary => {}
        }

        match sql_type {
            SqlType::Text => Ok(SqlValue::Text(Decode::<Postgres>::decode(value)?)),
            SqlType::Date => {
                let days: i32 = Decode::<Postgres>::decode(value)?;
                let days = i64::from(days) + POSTGRES_EPOCH_DAYS_FROM_UNIX_EPOCH;
                let date = SqlDate::from_days_since_unix_epoch(days)
                    .ok_or("PostgreSQL DATE is out of range")?;
                Ok(SqlValue::Date(date))
            }
            SqlType::Time => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                let micros =
                    u64::try_from(micros).map_err(|_| "PostgreSQL TIME is out of range")?;
                let time = SqlTime::from_microseconds_since_midnight(micros)
                    .ok_or("PostgreSQL TIME is out of range")?;
                Ok(SqlValue::Time(time))
            }
            SqlType::Timestamp => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                let micros = i128::from(micros) + POSTGRES_EPOCH_MICROS_FROM_UNIX_EPOCH;
                let datetime = SqlDateTime::from_microseconds_since_unix_epoch(micros)
                    .ok_or("PostgreSQL TIMESTAMP is out of range")?;
                Ok(SqlValue::Timestamp(datetime))
            }
            SqlType::TimestampTz => {
                let micros: i64 = Decode::<Postgres>::decode(value)?;
                let micros = i128::from(micros) + POSTGRES_EPOCH_MICROS_FROM_UNIX_EPOCH;
                let micros = i64::try_from(micros)?;
                Ok(SqlValue::TimestampTz(SqlTimestamp::from_unix_microseconds(
                    micros,
                )))
            }
        }
    }
}
