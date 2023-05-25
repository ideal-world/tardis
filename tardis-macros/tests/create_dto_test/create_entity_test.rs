use std::fmt::Write;
use tardis::basic::dto::TardisContext;
use tardis::chrono::Utc;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::IndexCreateStatement;
use tardis::db::sea_orm::*;
use tardis::serde_json::Value;
use tardis::{chrono, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(column_name = "number8")]
    pub number_i8_for_test: i8,
    pub number16: i16,
    #[index(index_id = "index_id_2")]
    pub number32: i32,
    #[index(unique, name = "index_1")]
    pub number64: i64,
    pub number_f32: f32,
    pub number_f64: f64,
    pub number_u8: Vec<u8>,
    #[index]
    pub can_bool: bool,
    #[index(full_text, index_id = "index_id_3")]
    pub can_be_null: Option<String>,
    #[sea_orm(custom_type = "char", custom_len = "50")]
    pub be_50_char: String,
    #[sea_orm(custom_type = "bit", custom_len = "1")]
    pub be_bit: bool,
    #[sea_orm(custom_type = "text")]
    pub be_text: String,
    pub be_json: Value,
    pub create_time: chrono::DateTime<Utc>,
    #[index(index_id = "index_id_2", index_type = "Custom(GiST)", full_text)]
    #[fill_ctx(own_paths)]
    pub aaa: String,
}

struct GiST;

impl Iden for GiST {
    fn unquoted(&self, s: &mut dyn Write) {
        s.write_str("GiST").expect("TODO: panic message");
    }
}

#[allow(dead_code)]
fn main() {
    let create_table_statement = ActiveModel::create_table_statement(DbBackend::Postgres);

    let table_name: Option<_> = create_table_statement.get_table_name();
    assert!(table_name.is_some());
    assert_eq!(format!("{:?}", table_name.unwrap()), "Table(tests)".to_string());

    let table_cols: &Vec<_> = create_table_statement.get_columns();
    assert_eq!(table_cols.len(), 16);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_name() == "id" && col.get_column_type() == Some(&ColumnType::String(None))).collect();
    assert_eq!(find_id.len(), 1);

    let create_indexes: Vec<IndexCreateStatement> = ActiveModel::create_index_statement();
    assert_eq!(create_indexes.len(), 3);
    let find_index_1: Vec<_> = create_indexes.iter().filter(|index| index.get_index_spec().get_column_names().contains(&"number64".to_string())).collect();
    assert_eq!(find_index_1.len(), 1);
    assert_eq!(find_index_1.first().unwrap().get_index_spec().get_column_names().len(), 2);
}
