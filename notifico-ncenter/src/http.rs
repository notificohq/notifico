use crate::entity::ncenter_notification;
use crate::entity::prelude::NcenterNotification;
use crate::NCenterPlugin;
use axum::routing::get;
use axum::{Extension, Json, Router};
use chrono::{TimeZone, Utc};
use notifico_core::http::AuthorizedRecipient;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, QueryOrder};
use std::sync::Arc;
use uuid::Uuid;

pub fn get_router<S: Clone + Send + Sync + 'static>(ncenter: Arc<NCenterPlugin>) -> Router<S> {
    Router::new()
        .route("/notifications", get(notifications))
        .layer(Extension(ncenter))
}

#[derive(serde::Serialize)]
struct Notification {
    id: Uuid,
    content: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
}

async fn notifications(
    Extension(recipient): Extension<Arc<AuthorizedRecipient>>,
    Extension(ncenter): Extension<Arc<NCenterPlugin>>,
) -> Json<Vec<Notification>> {
    let notifications: Vec<ncenter_notification::Model> = NcenterNotification::find()
        .filter(ncenter_notification::Column::RecipientId.eq(recipient.recipient_id))
        .filter(ncenter_notification::Column::ProjectId.eq(recipient.project_id))
        .order_by_desc(ncenter_notification::Column::CreatedAt)
        .all(&ncenter.db)
        .await
        .unwrap();

    Json(
        notifications
            .into_iter()
            .map(|m| Notification {
                id: m.id,
                content: m.content,
                created_at: Utc.from_utc_datetime(&m.created_at),
            })
            .collect(),
    )
}
