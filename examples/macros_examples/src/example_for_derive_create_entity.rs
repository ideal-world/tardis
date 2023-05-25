use std::fmt::Write;
use tardis::basic::dto::TardisContext;
use tardis::chrono::Utc;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::*;
use tardis::{chrono, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

// run `cargo expand example_for_derive_create_entity > derive_create_entity_expand.rs` \
// to see automatically impl TardisActiveModel
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
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
    // pub number_f32: f32,
    // pub number_f64: f64,
    pub number_u8: Vec<u8>,
    pub can_bool: bool,
    #[index(full_text, index_id = "index_id_3")]
    pub can_be_null: Option<String>,
    #[sea_orm(custom_type = "char", custom_len = "50")]
    pub be_50_char: String,
    #[sea_orm(custom_type = "bit", custom_len = "1")]
    pub be_bit: bool,
    pub be_text: String,
    pub be_uuid: String,
    pub create_time: chrono::DateTime<Utc>,
    #[index(index_id = "index_id_2", index_type = "Custom(GiST)", full_text)]
    #[fill_ctx(own_paths)]
    pub aaa: String,
}

// impl ActiveModelBehavior for ActiveModel {}

// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
// pub enum Relation {}
// macro_rules! derive_all {
//     () => {
//         #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
//         pub enum Relation {}
//     };
// }
// derive_all!();

struct GiST;

impl Iden for GiST {
    fn unquoted(&self, s: &mut dyn Write) {
        s.write_str("GiST").expect("TODO: panic message");
    }
}
