use std::fmt::Write;
use tardis::chrono::Utc;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::sea_query::IndexCreateStatement;
use tardis::db::sea_orm::sea_query::StringLen;
use tardis::db::sea_orm::*;
use tardis::serde_json::{self, Value};
use tardis::{chrono, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(ignore)]
    pub be_ignore: String,
    #[sea_orm(column_name = "number8")]
    pub number_i8_for_test: i8,
    pub number16: i16,
    #[index(index_id = "index_id_2")]
    pub number32: i32,
    #[index(unique, name = "index_1")]
    pub number64: i64,
    pub number_f32: f32,
    pub number_f64: f64,
    pub be_var_binary: Vec<u8>,
    #[tardis_entity(custom_len = "50")]
    pub be_option_50_var_binary: Option<Vec<u8>>,
    #[index]
    pub be_bool: bool,
    #[index(full_text, index_id = "index_id_3")]
    pub can_be_null: Option<String>,
    #[tardis_entity(custom_type = "char", custom_len = "50")]
    pub be_50_char: String,
    #[tardis_entity(custom_type = "bit", custom_len = "1")]
    pub be_bit: bool,
    #[tardis_entity(custom_type = "text")]
    pub be_text: String,
    pub be_json_1: Value,
    pub create_time: chrono::DateTime<Utc>,

    #[tardis_entity(custom_type = "array.string(50)", custom_len = "1")]
    pub be_custom_array_string: Vec<String>,

    pub be_vec_i8: Vec<i8>,
    pub be_option_vec_i8: Option<Vec<i8>>,
    pub be_vec_text: Vec<String>,
    #[tardis_entity(custom_len = "50")]
    pub be_option_vec_text: Option<Vec<String>>,

    pub be_custom: KeyValue,
    pub be_option_custom: Option<KeyValue>,
    // pub be_vec_custom: Vec<KeyValue>,
    // pub be_option_vec_custom: Option<Vec<KeyValue>>,
    #[index(index_id = "index_id_2", index_type = "Custom(GiST)", full_text)]
    #[fill_ctx(fill = "own_paths")]
    pub aaa: String,
}

struct GiST;

impl Iden for GiST {
    fn unquoted(&self, s: &mut dyn Write) {
        s.write_str("GiST").expect(" panic message");
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, sea_orm::FromJsonQueryResult)]
pub struct KeyValue {
    pub id: i32,
    pub name: String,
    pub price: f32,
    pub notes: Option<String>,
}

#[allow(dead_code)]
fn main() {
    let create_table_statement = ActiveModel::create_table_statement(DbBackend::Postgres);

    let table_name: Option<_> = create_table_statement.get_table_name();
    assert!(table_name.is_some());
    assert_eq!(format!("{:?}", table_name.unwrap()), "Table(SeaRc(tests))".to_string());

    let table_cols: &Vec<_> = create_table_statement.get_columns();
    assert_eq!(table_cols.len(), 24);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_name() == "id" && col.get_column_type() == Some(&ColumnType::String(StringLen::None))).collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_name() == "number8" && col.get_column_type() == Some(&ColumnType::TinyInteger)).collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> =
        table_cols.iter().filter(|col| col.get_column_name() == "be_var_binary" && col.get_column_type() == Some(&ColumnType::VarBinary(StringLen::None))).collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> =
        table_cols.iter().filter(|col| col.get_column_name() == "be_option_50_var_binary" && col.get_column_type() == Some(&ColumnType::VarBinary(StringLen::N(50)))).collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> = table_cols
        .iter()
        .filter(|col| {
            col.get_column_name() == "be_custom_array_string" && col.get_column_type() == Some(&ColumnType::Array(std::sync::Arc::new(ColumnType::String(StringLen::N(50)))))
        })
        .collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> = table_cols
        .iter()
        .filter(|col| col.get_column_name() == "be_vec_text" && col.get_column_type() == Some(&ColumnType::Array(std::sync::Arc::new(ColumnType::String(StringLen::None)))))
        .collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> = table_cols
        .iter()
        .filter(|col| col.get_column_name() == "be_option_vec_text" && col.get_column_type() == Some(&ColumnType::Array(std::sync::Arc::new(ColumnType::String(StringLen::N(50))))))
        .collect();
    assert_eq!(find_id.len(), 1);

    let create_indexes: Vec<IndexCreateStatement> = ActiveModel::create_index_statement();
    assert_eq!(create_indexes.len(), 4);
    let find_index_1: Vec<_> = create_indexes.iter().filter(|index| index.get_index_spec().get_column_names().contains(&"number32".to_string())).collect();
    assert_eq!(find_index_1.len(), 1);
    assert_eq!(find_index_1.first().unwrap().get_index_spec().get_column_names().len(), 2);
}
