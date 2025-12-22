use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, NaiveDateTime, Utc};
use sea_query::{JoinType, Order, SimpleExpr, Value, Values};
use wasi_sql::types::{DataType, Row};

pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<DataType>,
}

// A helper trait to map types to your specific converter functions
pub trait FetchValue: Sized {
    fn fetch(row: &Row, col: &str) -> anyhow::Result<Self>;
}

#[macro_export]
macro_rules! sql_model {
    (
        // Parse the Table Name
        table = $table:literal,

        // Parse the struct attributes (like #[derive(...)])
        $(#[$meta:meta])* // Parse visibility, struct name, and open brace
        pub struct $struct_name:ident {
            // Parse fields: "pub name : Type,"
            $(pub $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        // Generate the Struct Definition
        $(#[$meta])*
        pub struct $struct_name {
            $(pub $field_name : $field_type),*
        }

        // Implement the SqlModel Trait
        impl SqlModel for $struct_name {
            const TABLE: &'static str = $table;

            fn projection() -> &'static [&'static str] {
                // 'stringify!' converts the field identifier to a string literal
                &[ $( stringify!($field_name) ),* ]
            }

            fn from_row(row: &Row) -> anyhow::Result<Self> {
                Ok(Self {
                    $(
                        // Call the generic FetchValue trait defined above
                        $field_name: <$field_type as FetchValue>::fetch(row, stringify!($field_name))?,
                    )*
                })
            }
        }
    };
}

pub trait SqlModel: Sized {
    const TABLE: &'static str;

    fn projection() -> &'static [&'static str];

    fn ordering() -> Vec<OrderSpec> {
        Vec::new()
    }

    fn joins() -> Vec<JoinSpec> {
        Vec::new()
    }

    fn from_row(row: &Row) -> Result<Self>;
}

#[derive(Clone)]
pub struct JoinSpec {
    pub table: &'static str,
    pub alias: Option<&'static str>,
    pub on: SimpleExpr,
    pub kind: JoinType,
}

#[derive(Clone)]
pub struct OrderSpec {
    pub table: Option<&'static str>,
    pub column: &'static str,
    pub order: Order,
}

// Outbound conversion
pub fn values_to_wasi_datatypes(values: Values) -> Result<Vec<DataType>> {
    values.into_iter().map(value_to_wasi_datatype).collect()
}

fn value_to_wasi_datatype(value: Value) -> Result<DataType> {
    let data_type = match value {
        Value::Bool(v) => DataType::Boolean(v),
        Value::TinyInt(v) => DataType::Int32(v.map(|value| value as i32)),
        Value::SmallInt(v) => DataType::Int32(v.map(i32::from)),
        Value::Int(v) => DataType::Int32(v),
        Value::BigInt(v) => DataType::Int64(v),
        Value::TinyUnsigned(v) => DataType::Uint32(v.map(|value| value as u32)),
        Value::SmallUnsigned(v) => DataType::Uint32(v.map(u32::from)),
        Value::Unsigned(v) => DataType::Uint32(v),
        Value::BigUnsigned(v) => DataType::Uint64(v),
        Value::Float(v) => DataType::Float(v),
        Value::Double(v) => DataType::Double(v),
        Value::String(v) => DataType::Str(v.map(|value| *value)),
        Value::ChronoDate(v) => DataType::Date(v.map(|value| {
            let date = *value;
            date.to_string() // "%Y-%m-%d"
        })),
        Value::ChronoTime(v) => DataType::Time(v.map(|value| {
            let time = *value;
            time.to_string() // "%H:%M:%S%.f"
        })),
        Value::ChronoDateTime(v) => DataType::Timestamp(v.map(|value| {
            let dt = *value;
            dt.to_string() // "%Y-%m-%d %H:%M:%S%.f"
        })),
        Value::ChronoDateTimeUtc(v) => DataType::Timestamp(v.map(|value| {
            let dt: DateTime<Utc> = *value;
            dt.to_rfc3339() // "%Y-%m-%dT%H:%M:%S%.f%:z"
        })),
        Value::Char(v) => DataType::Str(v.map(|ch| ch.to_string())),
        Value::Bytes(v) => DataType::Binary(v.map(|bytes| *bytes)),
        _ => {
            bail!("unsupported values require explicit conversion before building the query")
        }
    };
    Ok(data_type)
}

// Inbound conversion
impl FetchValue for DateTime<Utc> {
    fn fetch(row: &Row, col: &str) -> anyhow::Result<Self> {
        as_timestamp(row_field(row, col)?)
    }
}

impl FetchValue for String {
    fn fetch(row: &Row, col: &str) -> anyhow::Result<Self> {
        as_string(row_field(row, col)?)
    }
}

impl FetchValue for serde_json::Value {
    fn fetch(row: &Row, col: &str) -> anyhow::Result<Self> {
        as_json(row_field(row, col)?)
    }
}

fn row_field<'a>(row: &'a Row, name: &str) -> Result<&'a DataType> {
    row.fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
        .ok_or_else(|| anyhow!("missing column '{name}'"))
}

fn as_string(value: &DataType) -> Result<String> {
    match value {
        DataType::Str(Some(raw)) => Ok(raw.clone()),
        _ => bail!("expected string data type"),
    }
}

fn as_timestamp(value: &DataType) -> Result<DateTime<Utc>> {
    match value {
        DataType::Timestamp(Some(raw)) => {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
                return Ok(parsed.with_timezone(&Utc));
            }

            if let Ok(parsed) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S%.f") {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(parsed, Utc));
            }

            bail!("unsupported timestamp: {}", raw)
        }
        _ => bail!("expected timestamp data type"),
    }
}

fn as_json(value: &DataType) -> Result<serde_json::Value> {
    match value {
        DataType::Str(Some(raw)) => Ok(serde_json::from_str(raw)?),
        DataType::Binary(Some(bytes)) => Ok(serde_json::from_slice(bytes)?),
        _ => bail!("expected json compatible data type"),
    }
}
