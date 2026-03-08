use sea_orm_migration::prelude::*;

mod m20260303_000001_create_projects;
mod m20260303_000002_create_events;
mod m20260303_000003_create_templates;
mod m20260303_000004_create_recipients;
mod m20260303_000005_create_delivery_log;
mod m20260303_000006_create_api_keys;
mod m20260303_000007_create_idempotency;
mod m20260304_000008_create_delivery_task;
mod m20260304_000009_create_credentials;
mod m20260308_000010_create_pipeline_middleware;
mod m20260308_000011_add_rule_id_to_delivery_task;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260303_000001_create_projects::Migration),
            Box::new(m20260303_000002_create_events::Migration),
            Box::new(m20260303_000003_create_templates::Migration),
            Box::new(m20260303_000004_create_recipients::Migration),
            Box::new(m20260303_000005_create_delivery_log::Migration),
            Box::new(m20260303_000006_create_api_keys::Migration),
            Box::new(m20260303_000007_create_idempotency::Migration),
            Box::new(m20260304_000008_create_delivery_task::Migration),
            Box::new(m20260304_000009_create_credentials::Migration),
            Box::new(m20260308_000010_create_pipeline_middleware::Migration),
            Box::new(m20260308_000011_add_rule_id_to_delivery_task::Migration),
        ]
    }
}
