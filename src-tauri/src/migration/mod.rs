pub use sea_orm_migration::prelude::*;

mod m20260831_000001_initial;
mod m20260902_000002_auth;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260831_000001_initial::Migration),
            Box::new(m20260902_000002_auth::Migration),
        ]
    }
}
