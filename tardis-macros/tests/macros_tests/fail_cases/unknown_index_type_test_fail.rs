use tardis::db::sea_orm::{self, *};
use tardis::{TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[tardis_entity(primary_key)]
    pub id: String,
    #[tardis_entity(custom_type = "unknown_type", custom_len = "[1]")]
    pub aaa: String,
}

#[allow(dead_code)]
fn main() {}
