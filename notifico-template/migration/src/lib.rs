#![allow(unused_imports)]
pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20241209_225154_drop_channel;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20241209_225154_drop_channel::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("template_migrations").into_iden()
    }
}
