use tardis::db::sea_orm::{self, *};
use tardis::{TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    pub be_custom_error: KeyValue,
    pub be_option_custom_error: Option<KeyValue>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyValue {
    pub id: i32,
    pub name: String,
    pub price: f32,
    pub notes: Option<String>,
}

#[allow(dead_code)]
fn main() {}
