use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm::*;
use tardis::TardisFuns;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "test_tenant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::tenant_conf::Entity")]
    TenantConfig,
    #[sea_orm(has_many = "super::app::Entity")]
    App,
}

impl Related<super::tenant_conf::Entity> for super::tenant::Entity {
    fn to() -> RelationDef {
        Relation::TenantConfig.def()
    }
}

impl Related<super::app::Entity> for super::tenant::Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl TardisActiveModel for ActiveModel {
    fn fill_cxt(&mut self, _: &TardisContext, is_insert: bool) {
        if is_insert {
            self.id = Set(TardisFuns::field.nanoid());
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
