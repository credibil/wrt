use std::error::Error;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use tokio_postgres::types::private::BytesMut;
use tokio_postgres::types::{IsNull, ToSql, Type, to_sql_checked};

pub type Param = Box<dyn ToSql + Send + Sync>;
pub type ParamRef<'a> = &'a (dyn ToSql + Sync);

/// `PgType` to wrap around wasi-sql `DataType` to help implement `ToSql` trait
#[derive(Debug)]
pub enum PgType {
    Int32(Option<i32>),
    Int64(Option<i64>),
    Uint32(Option<u32>),
    Float(Option<f32>),
    Double(Option<f64>),
    Text(Option<String>),
    Bool(Option<bool>),
    Date(Option<NaiveDate>),
    Time(Option<NaiveTime>),
    Timestamp(Option<NaiveDateTime>),
    TimestampTz(Option<DateTime<Utc>>),
    Binary(Option<Vec<u8>>),
}

impl ToSql for PgType {
    to_sql_checked!();

    fn to_sql(
        &self, ty: &Type, out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Send + Sync>> {
        match self {
            Self::Int32(value) => write_optional(value.as_ref(), ty, out),
            Self::Int64(value) => write_optional(value.as_ref(), ty, out),
            Self::Uint32(value) => write_optional(value.as_ref(), ty, out),
            Self::Float(value) => write_optional(value.as_ref(), ty, out),
            Self::Double(value) => write_optional(value.as_ref(), ty, out),
            Self::Text(value) => {
                if *ty == Type::JSON || *ty == Type::JSONB {
                    match value.as_ref() {
                        Some(raw) => {
                            let parsed: serde_json::Value = serde_json::from_str(raw)?;
                            tokio_postgres::types::Json(parsed).to_sql(ty, out)
                        }
                        None => Ok(IsNull::Yes),
                    }
                } else {
                    write_optional(value.as_ref(), ty, out)
                }
            }
            Self::Bool(value) => write_optional(value.as_ref(), ty, out),
            Self::Date(value) => write_optional(value.as_ref(), ty, out),
            Self::Time(value) => write_optional(value.as_ref(), ty, out),
            Self::Timestamp(value) => write_optional(value.as_ref(), ty, out),
            Self::TimestampTz(value) => write_optional(value.as_ref(), ty, out),
            Self::Binary(value) => write_optional(value.as_ref(), ty, out),
        }
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    fn encode_format(&self, _ty: &Type) -> tokio_postgres::types::Format {
        tokio_postgres::types::Format::Binary
    }
}

fn write_optional<T>(
    value: Option<&T>, ty: &Type, out: &mut BytesMut,
) -> Result<IsNull, Box<dyn Error + Send + Sync>>
where
    T: ToSql + Sync,
{
    value.map_or_else(|| Ok(IsNull::Yes), |inner| inner.to_sql(ty, out))
}
