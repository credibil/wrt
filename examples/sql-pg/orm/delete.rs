#![allow(dead_code)]

use std::marker::PhantomData;

use anyhow::Result;
use sea_query::{Alias, PostgresQueryBuilder, Query, SimpleExpr};

use crate::orm::query::{BuiltQuery, SqlModel, values_to_wasi_datatypes};

pub struct DeleteBuilder<M: SqlModel> {
    filters: Vec<SimpleExpr>,
    returning: Vec<&'static str>,
    _marker: PhantomData<M>,
}

impl<M: SqlModel> Default for DeleteBuilder<M> {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
            returning: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<M: SqlModel> DeleteBuilder<M> {
    pub fn new() -> Self {
        Self::default()
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
        let mut statement = Query::delete();
        statement.from_table(Alias::new(M::TABLE));

        for filter in self.filters {
            statement.and_where(filter);
        }

        for column in self.returning {
            statement.returning_col(Alias::new(column));
        }

        let (sql, values) = statement.build(PostgresQueryBuilder);
        let params = values_to_wasi_datatypes(values)?;
        Ok(BuiltQuery { sql, params })
    }
}
