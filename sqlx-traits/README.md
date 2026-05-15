# sqlx-traits

`sqlx-traits` defines small semantic SQL traits that data structure crates can
implement without depending on SQLx driver crates.

The goal is the same separation that Serde gives serialization formats and data
types:

- A data type crate implements `SqlEncode` and `SqlDecode`.
- A database driver maps semantic `SqlValue`s to its wire protocol.
- Application code can pass the data type through SQLx without a dedicated
  adapter crate for every data-type/database pair.

## Dependency footprint

`sqlx-traits` has no external crate dependencies. Its `[dependencies]` section
is intentionally empty.

The crate currently uses `std` for `String` and boxed errors. It does not depend
on SQLx itself, any SQLx driver crate, Jiff, Chrono, Time, Serde, or a runtime.

## Application usage

When a data type crate implements these traits, use SQLx's semantic binding and
decoding helpers:

```rust,no_run
use sqlx::Row;

# async fn example(pool: sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
let created_at: jiff::Timestamp = "2026-05-15T12:00:00Z".parse()?;

sqlx::query("insert into events (created_at) values ($1)")
    .bind_sql(created_at)
    .execute(&pool)
    .await?;

let row = sqlx::query("select created_at from events limit 1")
    .fetch_one(&pool)
    .await?;

let created_at: jiff::Timestamp = row.get_sql("created_at");
# Ok(())
# }
```

This requires:

- SQLx support for the target database's semantic bridge.
- A data type crate feature that implements `sqlx-traits`, for example Jiff's
  experimental `sqlx-traits` feature in this workspace.

## Implementing a type

Data type crates implement three traits:

```rust
use sqlx_traits::{
    BoxDynError, HasSqlType, SqlDate, SqlDecode, SqlEncode, SqlType, SqlValue,
    unexpected_sql_value,
};

pub struct Birthday {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl HasSqlType for Birthday {
    const SQL_TYPE: SqlType = SqlType::Date;
}

impl SqlEncode for Birthday {
    fn encode_sql(&self) -> Result<SqlValue, BoxDynError> {
        Ok(SqlValue::Date(SqlDate::from_ymd_unchecked(
            self.year, self.month, self.day,
        )))
    }
}

impl SqlDecode for Birthday {
    fn decode_sql(value: SqlValue) -> Result<Self, BoxDynError> {
        match value {
            SqlValue::Date(date) => Ok(Birthday {
                year: date.year,
                month: date.month,
                day: date.day,
            }),
            other => Err(unexpected_sql_value("date", &other)),
        }
    }
}
```

`HasSqlType::SQL_TYPE` declares the canonical semantic SQL type. `SqlEncode`
converts a Rust value into a `SqlValue`. `SqlDecode` converts a `SqlValue` back
into the Rust type.

## Semantic types

The current experiment includes:

- `SqlType::Text`
- `SqlType::Date`
- `SqlType::Time`
- `SqlType::Timestamp`
- `SqlType::TimestampTz`

The concrete value structs are intentionally database-independent:

- `SqlDate` is a Gregorian date.
- `SqlTime` is a time of day with nanosecond precision.
- `SqlDateTime` is a date and time without an offset or time zone.
- `SqlTimestamp` is an instant represented as nanoseconds since the Unix epoch.

Database drivers are responsible for mapping these semantic values to the
database's actual representation, including any precision loss. For example,
PostgreSQL stores timestamp values at microsecond precision, so sub-microsecond
nanoseconds are truncated when encoding through the PostgreSQL bridge.
