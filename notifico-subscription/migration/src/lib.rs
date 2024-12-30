pub use sea_orm_migration::prelude::*;

mod m20241224_225755_recipient;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20241224_225755_recipient::Migration)]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("subscription_migrations").into_iden()
    }
}
