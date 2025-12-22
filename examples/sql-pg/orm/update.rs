#![allow(dead_code)]

use std::marker::PhantomData;

use anyhow::Result;
use sea_query::{Alias, PostgresQueryBuilder, Query, SimpleExpr, Value};

use crate::orm::query::{BuiltQuery, SqlModel, values_to_wasi_datatypes};

pub struct UpdateBuilder<M: SqlModel> {
    set_clauses: Vec<(&'static str, Value)>,
    filters: Vec<SimpleExpr>,
    returning: Vec<&'static str>,
    _marker: PhantomData<M>,
}

impl<M: SqlModel> Default for UpdateBuilder<M> {
    fn default() -> Self {
        Self {
            set_clauses: Vec::new(),
            filters: Vec::new(),
            returning: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<M: SqlModel> UpdateBuilder<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<V>(mut self, column: &'static str, value: V) -> Self
    where
        V: Into<Value>,
    {
        self.set_clauses.push((column, value.into()));
        self
    }

    pub fn r#where(mut self, expr: SimpleExpr) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn returning(mut self, column: &'static str) -> Self {
        self.returning.push(column);
        self
    }

    pub fn build(self) -> Result<BuiltQuery> {
        let mut statement = Query::update();
        statement.table(Alias::new(M::TABLE));

        for (column, value) in self.set_clauses {
            statement.value(Alias::new(column), value);
        }

        for expr in self.filters {
            statement.and_where(expr);
        }

        for column in self.returning {
            statement.returning_col(Alias::new(column));
        }

        let (sql, values) = statement.build(PostgresQueryBuilder);
        let params = values_to_wasi_datatypes(values)?;
        Ok(BuiltQuery { sql, params })
    }
}
