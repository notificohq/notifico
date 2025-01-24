//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "event")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::pipeline_event_j::Entity")]
    PipelineEventJ,
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id",
        on_update = "Restrict",
        on_delete = "Cascade"
    )]
    Project,
}

impl Related<super::pipeline_event_j::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PipelineEventJ.def()
    }
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::pipeline::Entity> for Entity {
    fn to() -> RelationDef {
        super::pipeline_event_j::Relation::Pipeline.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::pipeline_event_j::Relation::Event.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
