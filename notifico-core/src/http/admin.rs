use anyhow::bail;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, EntityTrait, QueryOrder, QuerySelect, Select};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::str::FromStr;

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl From<SortOrder> for sea_orm::Order {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Asc => sea_orm::Order::Asc,
            SortOrder::Desc => sea_orm::Order::Desc,
        }
    }
}

pub trait ListableTrait: QuerySelect {
    fn apply_params(self, params: &ListQueryParams) -> anyhow::Result<Self>;
}

impl<ET> ListableTrait for Select<ET>
where
    ET: EntityTrait,
    <ET::Column as FromStr>::Err: Error + Send + Sync,
{
    fn apply_params(mut self, params: &ListQueryParams) -> anyhow::Result<Self> {
        if let Some(order) = &params.sort {
            let order: (String, SortOrder) = serde_json::from_str(order)?;

            self = self.order_by(ET::Column::from_str(&order.0)?, order.1.into())
        }
        if let Some(range) = &params.range {
            let range: (u64, u64) = serde_json::from_str(range)?;

            self = self.offset(range.0).limit(range.1 - range.0);
        }
        if let Some(filter) = &params.filter {
            let filter: BTreeMap<String, Value> = serde_json::from_str(filter)?;

            for (col, val) in filter.into_iter() {
                match val {
                    Value::String(v) => self = self.filter(ET::Column::from_str(&col)?.eq(v)),
                    Value::Array(v) => self = self.filter(ET::Column::from_str(&col)?.is_in(v)),
                    _ => {
                        bail!("Invalid filter value type: {col}. Expected string or array of strings.")
                    }
                }
            }
        }
        Ok(self)
    }
}

#[derive(Deserialize, Clone, Default)]
pub struct ListQueryParams {
    pub sort: Option<String>,
    pub range: Option<String>,
    pub filter: Option<String>,
}

pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total_count: u64,
}
