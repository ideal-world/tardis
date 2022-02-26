use poem_openapi::param::Query;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};

use tardis::db::reldb_client::TardisSeaORMExtend;
use tardis::db::sea_orm::*;
use tardis::db::sea_query::*;
use tardis::serde::{self, Deserialize, Serialize};
use tardis::web::web_resp::{TardisPage, TardisResp};
use tardis::TardisFuns;

use crate::domain::todos;

#[derive(Object, FromQueryResult, Serialize, Deserialize, Debug)]
#[serde(crate = "self::serde")]
struct TodoDetailResp {
    id: i32,
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug)]
#[serde(crate = "self::serde")]
struct TodoAddReq {
    #[oai(validator(min_length = "2", max_length = "255"))]
    description: String,
    done: bool,
}

#[derive(Object, Serialize, Deserialize, Debug)]
#[serde(crate = "self::serde")]
struct TodoModifyReq {
    #[oai(validator(min_length = "2", max_length = "255"))]
    description: Option<String>,
    done: Option<bool>,
}

pub struct TodoApi;

#[OpenApi]
impl TodoApi {
    #[oai(path = "/todo", method = "post")]
    async fn add(&self, todo_add_req: Json<TodoAddReq>) -> TardisResp<i32> {
        let todo = todos::ActiveModel {
            description: Set(todo_add_req.description.to_string()),
            done: Set(todo_add_req.done),
            ..Default::default()
        }
        .insert(TardisFuns::reldb().conn())
        .await
        .unwrap();
        TardisResp::ok(todo.id)
    }

    #[oai(path = "/todo/:id", method = "get")]
    async fn get(&self, id: Path<i32>) -> TardisResp<TodoDetailResp> {
        let todo_detail_resp = todos::Entity::find()
            .filter(todos::Column::Id.eq(id.0))
            .select_only()
            .column(todos::Column::Id)
            .column(todos::Column::Description)
            .column(todos::Column::Done)
            .into_model::<TodoDetailResp>()
            .one(TardisFuns::reldb().conn())
            .await
            .unwrap()
            .unwrap();
        TardisResp::ok(todo_detail_resp)
    }

    #[oai(path = "/todo", method = "get")]
    async fn get_all(&self, page_number: Query<usize>, page_size: Query<usize>) -> TardisResp<TardisPage<TodoDetailResp>> {
        let result = todos::Entity::find()
            .select_only()
            .column(todos::Column::Id)
            .column(todos::Column::Description)
            .column(todos::Column::Done)
            .order_by_desc(todos::Column::Id)
            .into_model::<TodoDetailResp>()
            .paginate(TardisFuns::reldb().conn(), page_size.0);
        TardisResp::ok(TardisPage {
            page_size: page_size.0,
            page_number: result.num_pages().await.unwrap(),
            total_size: result.num_items().await.unwrap(),
            records: result.fetch_page(page_number.0 - 1).await.unwrap(),
        })
    }

    #[oai(path = "/todo/:id", method = "delete")]
    async fn delete(&self, id: Path<i32>) -> TardisResp<usize> {
        let delete_num = todos::Entity::find().filter(todos::Column::Id.eq(id.0)).soft_delete(TardisFuns::reldb().conn(), "").await.unwrap();
        TardisResp::ok(delete_num)
    }

    #[oai(path = "/todo/:id", method = "put")]
    async fn update(&self, id: Path<i32>, todo_modify_req: Json<TodoModifyReq>) -> TardisResp<usize> {
        let mut update = todos::Entity::update_many().filter(todos::Column::Id.eq(id.0));
        if let Some(description) = &todo_modify_req.description {
            update = update.col_expr(todos::Column::Description, Expr::value(description.clone()));
        }
        if let Some(done) = todo_modify_req.done {
            update = update.col_expr(todos::Column::Done, Expr::value(done));
        }
        let update_num = update.exec(TardisFuns::reldb().conn()).await.unwrap().rows_affected;
        TardisResp::ok(update_num as usize)
    }
}
