//! Adapter support for semantic SQL type traits.

use std::fmt;

use crate::database::Database;
use crate::decode::Decode;
use crate::encode::{Encode, IsNull};
use crate::error::BoxDynError;
use crate::type_info::TypeInfo;
use crate::types::Type;
use crate::value::ValueRef;

pub use sqlx_traits::{
    HasSqlType, SqlDate, SqlDateTime, SqlDecode, SqlEncode, SqlTime, SqlTimestamp, SqlType,
    SqlValue,
};

/// A local adapter that lets SQLx's existing `Encode`, `Decode` and `Type`
/// traits work with semantic SQL values.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Sql<T>(pub T);

/// Database support for values described by `sqlx-traits`.
pub trait DatabaseSqlTypes: Database {
    /// Returns the driver-specific type info for a semantic SQL type.
    fn sql_type_info(sql_type: SqlType) -> Self::TypeInfo;

    /// Returns whether a driver-specific type is compatible with a semantic SQL type.
    fn sql_type_compatible(sql_type: SqlType, type_info: &Self::TypeInfo) -> bool {
        Self::sql_type_info(sql_type).type_compatible(type_info)
    }

    /// Encodes a semantic SQL value into the driver's argument buffer.
    fn encode_sql_value(
        value: SqlValue,
        buf: &mut Self::ArgumentBuffer,
    ) -> Result<IsNull, BoxDynError>;

    /// Decodes a semantic SQL value from the driver's value reference.
    fn decode_sql_value(
        sql_type: SqlType,
        value: Self::ValueRef<'_>,
    ) -> Result<SqlValue, BoxDynError>;
}

impl<T, DB> Type<DB> for Sql<T>
where
    T: HasSqlType,
    DB: DatabaseSqlTypes,
{
    fn type_info() -> DB::TypeInfo {
        DB::sql_type_info(T::SQL_TYPE)
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        ty.is_null() || DB::sql_type_compatible(T::SQL_TYPE, ty)
    }
}

impl<'q, T, DB> Encode<'q, DB> for Sql<T>
where
    T: SqlEncode,
    DB: DatabaseSqlTypes,
{
    fn encode_by_ref(&self, buf: &mut DB::ArgumentBuffer) -> Result<IsNull, BoxDynError> {
        DB::encode_sql_value(self.0.encode_sql()?, buf)
    }

    fn produces(&self) -> Option<DB::TypeInfo> {
        Some(DB::sql_type_info(T::SQL_TYPE))
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<SqlValue>()
    }
}

impl<'r, T, DB> Decode<'r, DB> for Sql<T>
where
    T: SqlDecode,
    DB: DatabaseSqlTypes,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = if value.is_null() {
            SqlValue::Null
        } else {
            DB::decode_sql_value(T::SQL_TYPE, value)?
        };

        T::decode_sql(value).map(Sql)
    }
}

impl<T> fmt::Display for Sql<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
