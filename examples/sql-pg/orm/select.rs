#![allow(dead_code)]
use std::marker::PhantomData;

use anyhow::Result;
use sea_query::{Alias, ColumnRef, IntoIden, Order, Query, SimpleExpr};

use crate::orm::entity::{Entity, JoinSpec, values_to_wasi_datatypes};
use crate::orm::query::{BuiltQuery, OrmQueryBuilder};
use crate::provider::SqlDb;

pub struct SelectBuilder<M: Entity> {
    filters: Vec<SimpleExpr>,
    limit: Option<u64>,
    offset: Option<u64>,
    order: Vec<(ColumnRef, Order)>,
    joins: Vec<JoinSpec>,
    _marker: PhantomData<M>,
}

impl<M: Entity> Default for SelectBuilder<M> {
    fn default() -> Self {
        let ordering = M::ordering()
            .into_iter()
            .map(|spec| (table_column(spec.table.unwrap_or(M::TABLE), spec.column), spec.order))
            .collect();

        Self {
            filters: Vec::new(),
            limit: None,
            offset: None,
            order: ordering,
            joins: M::joins(),
            _marker: PhantomData,
        }
    }
}

impl<M: Entity> SelectBuilder<M> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn r#where(mut self, expr: SimpleExpr) -> Self {
        self.filters.push(expr);
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn order_by(mut self, table: Option<&'static str>, column: &'static str) -> Self {
        let table = table.unwrap_or(M::TABLE);
        self.order.push((table_column(table, column), Order::Asc));
        self
    }

    pub fn order_by_desc(mut self, table: Option<&'static str>, column: &'static str) -> Self {
        let table = table.unwrap_or(M::TABLE);
        self.order.push((table_column(table, column), Order::Desc));
        self
    }

    pub fn join(mut self, spec: JoinSpec) -> Self {
        self.joins.push(spec);
        self
    }

    /// Consumes the builder, executes the query against the provider, and maps rows to the Model.
    pub async fn fetch(self, provider: &impl SqlDb, pool_name: &str) -> Result<Vec<M>> {
        let BuiltQuery { sql, params } =
            self.build().map_err(|e| anyhow::anyhow!("failed building query: {e:?}"))?;

        let rows = provider
            .query(pool_name.to_string(), sql, params)
            .await
            .map_err(|e| anyhow::anyhow!("query failed: {e:?}"))?;

        let models = rows
            .iter()
            .map(M::from_row)
            .collect::<Result<Vec<_>>>()
            .map_err(|e| anyhow::anyhow!("row conversion failed: {e:?}"))?;

        Ok(models)
    }

    pub fn build(self) -> Result<BuiltQuery> {
        let mut statement = Query::select();
        let projection: Vec<ColumnRef> =
            M::projection().iter().map(|column| table_column(M::TABLE, column)).collect();

        statement.columns(projection).from(Alias::new(M::TABLE));

        for JoinSpec {
            table,
            alias,
            on,
            kind,
        } in self.joins
        {
            let table_alias = Alias::new(table);
            if let Some(alias) = alias {
                statement.join_as(kind, table_alias, Alias::new(alias), on);
            } else {
                statement.join(kind, table_alias, on);
            }
        }

        for filter in self.filters {
            statement.and_where(filter);
        }

        if let Some(limit) = self.limit {
            statement.limit(limit);
        }

        if let Some(offset) = self.offset {
            statement.offset(offset);
        }

        for (column, order) in self.order {
            statement.order_by(column, order);
        }

        let (sql, values) = statement.build(OrmQueryBuilder::default());
        let params = values_to_wasi_datatypes(values)?;
        Ok(BuiltQuery { sql, params })
    }
}

pub fn table_column(table: &str, column: &str) -> ColumnRef {
    ColumnRef::TableColumn(Alias::new(table).into_iden(), Alias::new(column).into_iden())
}
