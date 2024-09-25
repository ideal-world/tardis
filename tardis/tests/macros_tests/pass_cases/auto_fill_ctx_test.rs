use tardis::basic::dto::TardisContext;
use tardis::db::reldb_client::TardisActiveModel;
use tardis::db::sea_orm;
use tardis::db::sea_orm::*;
use tardis::{TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, TardisCreateEntity, TardisEmptyBehavior, TardisEmptyRelation)]
#[sea_orm(table_name = "tests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[fill_ctx(fill = "own_paths")]
    pub auto_fill_ctx: String,
}

#[allow(dead_code)]
fn main() {
    let create_table_statement = ActiveModel::create_table_statement(DbBackend::Postgres);
    let mut tests_model = ActiveModel {
        id: Set(String::new()),
        ..Default::default()
    };
    tests_model.fill_ctx(
        &TardisContext {
            own_paths: "own_paths".to_string(),
            ..Default::default()
        },
        true,
    );

    assert!(tests_model.auto_fill_ctx.is_set());
    assert_eq!(tests_model.auto_fill_ctx, Set("own_paths".to_string()));

    let table_name: Option<_> = create_table_statement.get_table_name();
    assert!(table_name.is_some());
    assert_eq!(format!("{:?}", table_name.unwrap()), "Table(SeaRc(tests))".to_string());

    let table_cols: &Vec<_> = create_table_statement.get_columns();
    assert_eq!(table_cols.len(), 2);
}
