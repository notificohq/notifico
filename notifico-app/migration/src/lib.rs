#![allow(unused_imports)]
pub use sea_orm_migration::prelude::*;

mod m20250108_000001_create_project_table;
mod m20250108_000002_create_pipeline_event;
mod m20250108_000003_recipient;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250108_000001_create_project_table::Migration),
            Box::new(m20250108_000002_create_pipeline_event::Migration),
            Box::new(m20250108_000003_recipient::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("notifico_migrations").into_iden()
    }
}
