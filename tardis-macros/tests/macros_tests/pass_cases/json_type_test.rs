use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::prelude::Json;
use tardis::db::sea_orm::*;
use tardis::serde_json::{self, Value};
use tardis::{TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    #[tardis_entity(primary_key)]
    pub id: String,
    pub be_json_1: Value,
    pub be_json_2: Json,

    pub be_custom: KeyValue,
    pub be_option_custom: Option<KeyValue>,
    // pub be_vec_custom: Vec<KeyValue>,
    // pub be_option_vec_custom: Option<Vec<KeyValue>>,
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
    assert_eq!(format!("{:?}", table_name.unwrap()), "Table(tests)".to_string());

    let table_cols: &Vec<_> = create_table_statement.get_columns();
    assert_eq!(table_cols.len(), 5);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_type() == Some(&ColumnType::Json)).collect();
    assert_eq!(find_id.len(), 4);
}
