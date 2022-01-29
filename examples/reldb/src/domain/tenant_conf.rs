use sea_orm::entity::prelude::*;
use sea_orm::ActiveModelBehavior;
use sea_orm::ActiveValue::Set;

use tardis::TardisFuns;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "test_tenant_conf")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub tenant_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::tenant::Entity", from = "Column::TenantId", to = "super::tenant::Column::Id")]
    Tenant,
}

impl Related<super::tenant::Entity> for super::tenant_conf::Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(TardisFuns::field.uuid_str()),
            ..ActiveModelTrait::default()
        }
    }
}
