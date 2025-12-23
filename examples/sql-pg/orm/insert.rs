#![allow(dead_code)]

use std::marker::PhantomData;

use anyhow::Result;
use sea_query::{Alias, Query, SimpleExpr, Value};

use crate::orm::entity::{Entity, values_to_wasi_datatypes};
use crate::orm::query::{BuiltQuery, OrmQueryBuilder};

pub struct InsertBuilder<M: Entity> {
    values: Vec<(&'static str, Value)>,
    _marker: PhantomData<M>,
}

impl<M: Entity> Default for InsertBuilder<M> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<M: Entity> InsertBuilder<M> {
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

    pub fn build(self) -> Result<BuiltQuery> {
        let mut statement = Query::insert();
        statement.into_table(Alias::new(M::TABLE));

        let columns: Vec<_> = self.values.iter().map(|(column, _)| Alias::new(*column)).collect();
        let row: Vec<SimpleExpr> =
            self.values.into_iter().map(|(_, value)| SimpleExpr::Value(value)).collect();

        statement.columns(columns);
        statement.values_panic(row);

        let (sql, values) = statement.build(OrmQueryBuilder::default());
        let params = values_to_wasi_datatypes(values)?;
        Ok(BuiltQuery { sql, params })
    }
}
