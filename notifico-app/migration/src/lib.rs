#![allow(unused_imports)]
pub use sea_orm_migration::prelude::*;

mod m20250108_000001_create_project_table;
mod m20250108_000002_create_pipeline_event;
mod m20250108_000003_recipient;
mod m20250121_000001_create_template_table;
mod m20250203_000001_create_apikey_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250108_000001_create_project_table::Migration),
            Box::new(m20250108_000002_create_pipeline_event::Migration),
            Box::new(m20250108_000003_recipient::Migration),
            Box::new(m20250121_000001_create_template_table::Migration),
            Box::new(m20250203_000001_create_apikey_table::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("notifico_migrations").into_iden()
    }
}
