use std::fmt::Write;
use tardis::chrono::Utc;
use tardis::db::sea_orm;
use tardis::db::sea_orm::*;
use tardis::serde_json;
use tardis::{chrono, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

// run `cargo expand example_for_derive_create_entity > derive_create_entity_expand.rs` \
// to see automatically impl TardisActiveModel
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "examples")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_name = "number8")]
    pub number_i8_for_test: i8,
    pub number16: i16,
    #[index(index_id = "index_id_2")]
    pub number32: i32,
    #[index(unique)]
    pub number64: i64,
    pub number_f32: f32,
    pub number_f64: f64,
    pub be_var_binary: Vec<u8>,
    pub can_bool: bool,
    #[index(full_text, index_id = "index_id_3")]
    pub can_be_null: Option<String>,
    #[tardis_entity(custom_type = "Char(50)", custom_len = "50")]
    pub be_50_char: String,
    #[tardis_entity(custom_type = "varbit(50)")]
    pub be_var_bit: Vec<u8>,
    #[tardis_entity(custom_type = "array.string(50))")]
    pub test_array: Vec<String>,
    #[tardis_entity(custom_type = "bit", custom_len = "1")]
    pub be_bit: bool,
    pub create_time: chrono::DateTime<Utc>,
    pub key_value: Option<KeyValue>,
    // pub key_values: Vec<KeyValue>,
    #[index(index_id = "index_id_2", index_type = "Custom(GiST)", full_text)]
    #[fill_ctx(fill = "own_paths")]
    pub own_paths: String,
    #[fill_ctx(insert_only = false)]
    pub update_by: String,
}

struct GiST;

impl Iden for GiST {
    fn unquoted(&self, s: &mut dyn Write) {
        s.write_str("GiST").expect("panic message");
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, sea_orm::FromJsonQueryResult)]
pub struct KeyValue {
    pub id: i32,
    pub name: String,
    pub price: f32,
    pub notes: Option<String>,
}
