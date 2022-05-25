use tardis::basic::dto::TardisContext;
use tardis::db::sea_orm::*;
use tardis::db::sea_query::Query as DbQuery;
use tardis::serde::{self, Deserialize, Serialize};
use tardis::web::poem_openapi::param::Query;
use tardis::web::poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use tardis::web::web_resp::{TardisApiResult, TardisPage, TardisResp};
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
    async fn add(&self, todo_add_req: Json<TodoAddReq>) -> TardisApiResult<i32> {
        let cxt = TardisContext {
            own_paths: "".to_string(),
            ak: "".to_string(),
            owner: "".to_string(),
            roles: vec![],
            groups: vec![],
        };
        let todo_id = TardisFuns::reldb()
            .conn()
            .insert_one(
                todos::ActiveModel {
                    description: Set(todo_add_req.description.to_string()),
                    done: Set(todo_add_req.done),
                    ..Default::default()
                },
                &cxt,
            )
            .await?
            .last_insert_id;
        TardisResp::ok(todo_id)
    }

    #[oai(path = "/todo/:id", method = "get")]
    async fn get(&self, id: Path<i32>) -> TardisApiResult<TodoDetailResp> {
        let todo = TardisFuns::reldb()
            .conn()
            .get_dto(DbQuery::select().columns(vec![todos::Column::Id, todos::Column::Description, todos::Column::Done]).from(todos::Entity).and_where(todos::Column::Id.eq(id.0)))
            .await?
            .unwrap();
        TardisResp::ok(todo)
    }

    #[oai(path = "/todo", method = "get")]
    async fn get_all(&self, page_number: Query<u64>, page_size: Query<u64>) -> TardisApiResult<TardisPage<TodoDetailResp>> {
        let (todos, total_size) = TardisFuns::reldb()
            .conn()
            .paginate_dtos(
                DbQuery::select().columns(vec![todos::Column::Id, todos::Column::Description, todos::Column::Done]).from(todos::Entity),
                page_number.0,
                page_size.0,
            )
            .await?;
        TardisResp::ok(TardisPage {
            page_number: page_number.0,
            page_size: page_size.0,
            total_size,
            records: todos,
        })
    }

    #[oai(path = "/todo/:id", method = "delete")]
    async fn delete(&self, id: Path<i32>) -> TardisApiResult<u64> {
        let delete_num = TardisFuns::reldb().conn().soft_delete(todos::Entity::find().filter(todos::Column::Id.eq(id.0)), "").await?;
        TardisResp::ok(delete_num)
    }

    #[oai(path = "/todo/:id", method = "put")]
    async fn update(&self, id: Path<i32>, todo_modify_req: Json<TodoModifyReq>) -> TardisApiResult<u64> {
        let cxt = TardisContext {
            own_paths: "".to_string(),
            ak: "".to_string(),
            owner: "".to_string(),
            roles: vec![],
            groups: vec![],
        };
        TardisFuns::reldb()
            .conn()
            .update_one(
                todos::ActiveModel {
                    id: Set(id.0),
                    description: todo_modify_req.description.as_ref().map(|v| Set(v.clone())).unwrap_or(NotSet),
                    done: todo_modify_req.done.map(Set).unwrap_or(NotSet),
                },
                &cxt,
            )
            .await?;
        TardisResp::ok(0)
    }
}
