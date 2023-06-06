use rust_decimal::Decimal;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::*;
use tardis::{TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[tardis_entity(custom_len = "10", custom_len = "2")]
    pub be_decimal: Decimal,
    #[tardis_entity(custom_type = "Money", custom_len = "10", custom_len = "2")]
    pub be_money: Decimal,
}

#[allow(dead_code)]
fn main() {
    let create_table_statement = ActiveModel::create_table_statement(DbBackend::Postgres);

    let table_name: Option<_> = create_table_statement.get_table_name();
    assert!(table_name.is_some());
    assert_eq!(format!("{:?}", table_name.unwrap()), "Table(tests)".to_string());

    let table_cols: &Vec<_> = create_table_statement.get_columns();
    assert_eq!(table_cols.len(), 3);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_name() == "be_decimal" && col.get_column_type() == Some(&ColumnType::Decimal(Some((10, 2))))).collect();
    assert_eq!(find_id.len(), 1);
    let find_id: Vec<_> = table_cols.iter().filter(|col| col.get_column_name() == "be_money" && col.get_column_type() == Some(&ColumnType::Money(Some((10, 2))))).collect();
    assert_eq!(find_id.len(), 1);
}
