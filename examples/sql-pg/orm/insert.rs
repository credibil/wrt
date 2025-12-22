#![allow(dead_code)]

use std::marker::PhantomData;

use anyhow::Result;
use sea_query::{Alias, PostgresQueryBuilder, Query, SimpleExpr, Value};

use crate::orm::query::{BuiltQuery, SqlModel, values_to_wasi_datatypes};

pub struct InsertBuilder<M: SqlModel> {
    values: Vec<(&'static str, Value)>,
    returning: Vec<&'static str>,
    _marker: PhantomData<M>,
}

impl<M: SqlModel> Default for InsertBuilder<M> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            returning: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<M: SqlModel> InsertBuilder<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<V>(mut self, column: &'static str, value: V) -> Self
    where
        V: Into<Value>,
    {
        self.values.push((column, value.into()));
        self
    }

    pub fn returning(mut self, column: &'static str) -> Self {
        self.returning.push(column);
        self
    }

    pub fn build(self) -> Result<BuiltQuery> {
        let mut statement = Query::insert();
        statement.into_table(Alias::new(M::TABLE));

        let columns: Vec<_> = self.values.iter().map(|(column, _)| Alias::new(*column)).collect();
        let row: Vec<SimpleExpr> =
            self.values.into_iter().map(|(_, value)| SimpleExpr::Value(value)).collect();

        statement.columns(columns);
        statement.values_panic(row);

        for column in self.returning {
            statement.returning_col(Alias::new(column));
        }

        let (sql, values) = statement.build(PostgresQueryBuilder);
        let params = values_to_wasi_datatypes(values)?;
        Ok(BuiltQuery { sql, params })
    }
}
