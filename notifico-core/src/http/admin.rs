use crate::error::EngineError;
use anyhow::bail;
use async_trait::async_trait;
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, Copy, Clone, ToSchema)]
pub enum SortOrder {
    #[serde(alias = "ASC", alias = "asc")]
    Asc,
    #[serde(alias = "DESC", alias = "desc")]
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
    fn apply_filter(self, params: &ParsedListQueryParams) -> anyhow::Result<Self>;
    fn apply_params(self, params: &ParsedListQueryParams) -> anyhow::Result<Self>;
}

impl<ET> ListableTrait for Select<ET>
where
    ET: EntityTrait,
    <ET::Column as FromStr>::Err: Error + Send + Sync,
{
    fn apply_filter(mut self, params: &ParsedListQueryParams) -> anyhow::Result<Self> {
        let Some(filter) = &params.filter else {
            return Ok(self);
        };

        for (column, filterop) in filter.iter() {
            let column = ET::Column::from_str(column)?;
            match filterop {
                FilterOp::IsIn(filter) => {
                    let mut values: Vec<sea_orm::Value> = vec![];
                    for filter in filter {
                        if let Ok(uuid) = Uuid::from_str(filter) {
                            values.push(uuid.into());
                        } else {
                            values.push(filter.into())
                        }
                    }
                    self = self.filter(column.is_in(values));
                }
            }
        }

        Ok(self)
    }

    fn apply_params(mut self, params: &ParsedListQueryParams) -> anyhow::Result<Self> {
        if let Some(order) = &params.sort {
            self = self.order_by(ET::Column::from_str(&order.0)?, order.1.into())
        }
        if let Some(limit) = params.limit() {
            self = self.limit(limit)
        }
        if let Some(offset) = params.offset() {
            self = self.offset(offset)
        }

        self = self.apply_filter(params)?;
        Ok(self)
    }
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum ListQueryParams {
    ReactAdmin(ReactAdminListQueryParams),
    Refine(RefineListQueryParams),
}

#[derive(Deserialize, Clone, IntoParams)]
#[serde(deny_unknown_fields)]
pub struct ReactAdminListQueryParams {
    sort: Option<String>,
    range: Option<String>,
    filter: Option<String>,
}

#[derive(Deserialize, Clone, IntoParams)]
pub struct RefineListQueryParams {
    _sort: Option<String>,
    #[param(inline)]
    _order: Option<SortOrder>,
    _start: Option<u64>,
    _end: Option<u64>,
    #[serde(flatten)]
    _filter: Vec<(String, String)>,
}

#[derive(Deserialize, Clone)]
enum FilterOp {
    IsIn(Vec<String>),
}

pub struct ParsedListQueryParams {
    range: Option<(u64, u64)>,
    filter: Option<Vec<(String, FilterOp)>>,
    sort: Option<(String, SortOrder)>,
}

impl ParsedListQueryParams {
    fn limit(&self) -> Option<u64> {
        self.range.map(|(start, end)| end - start)
    }

    fn offset(&self) -> Option<u64> {
        self.range.map(|(start, _)| start)
    }
}

impl TryFrom<ReactAdminListQueryParams> for ParsedListQueryParams {
    type Error = anyhow::Error;

    fn try_from(value: ReactAdminListQueryParams) -> Result<Self, Self::Error> {
        let sort = match value.sort {
            None => None,
            Some(sort) => serde_json::from_str(&sort)?,
        };

        let range = match value.range {
            None => None,
            Some(range) => serde_json::from_str(&range)?,
        };

        let filter = match value.filter {
            None => None,
            Some(filter) => {
                let mut parsed_filter = vec![];

                let filter: BTreeMap<String, Value> = serde_json::from_str(&filter)?;
                for (col, values) in filter.into_iter() {
                    let values = match values {
                        Value::String(v) => vec![v],
                        Value::Array(v) => {
                            let mut values: Vec<String> = vec![];
                            for filter in v {
                                match filter {
                                    Value::String(filter) => values.push(filter),
                                    _ => {
                                        bail!("Invalid filter value type: {col}. Expected string.")
                                    }
                                }
                            }
                            values
                        }
                        _ => {
                            bail!("Invalid filter value type: {col}. Expected string or array of strings.")
                        }
                    };
                    parsed_filter.push((col, FilterOp::IsIn(values)));
                }
                Some(parsed_filter)
            }
        };

        Ok(Self {
            range,
            filter,
            sort,
        })
    }
}

impl TryFrom<ListQueryParams> for ParsedListQueryParams {
    type Error = anyhow::Error;

    fn try_from(value: ListQueryParams) -> Result<Self, Self::Error> {
        match value {
            ListQueryParams::ReactAdmin(value) => value.try_into(),
            ListQueryParams::Refine(_) => unimplemented!(),
        }
    }
}

pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
}

impl<T> PaginatedResult<T> {
    pub fn map<P>(self, f: impl Fn(T) -> P) -> PaginatedResult<P> {
        PaginatedResult {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
        }
    }
}

impl<T: Serialize> IntoResponse for PaginatedResult<T> {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_RANGE, self.total.into());

        (headers, Json(self.items)).into_response()
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemWithId<T> {
    pub id: Uuid,
    #[serde(flatten)]
    pub item: T,
}

impl<T> ItemWithId<T> {
    pub fn map<U>(self, f: impl Fn(T) -> U) -> ItemWithId<U> {
        ItemWithId {
            id: self.id,
            item: f(self.item),
        }
    }
}

#[async_trait]
pub trait AdminCrudTable {
    type Item;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError>;
    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError>;
    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError>;
    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError>;
    async fn delete(&self, id: Uuid) -> Result<(), EngineError>;
}
