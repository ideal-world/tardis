use std::env;
use std::vec;

use tardis::basic::result::TardisResult;
use tardis::test::test_container::TardisTestContainer;
use tardis::tokio;
use tardis::TardisFuns;

#[tokio::main]
async fn main() -> TardisResult<()> {
    // Here is a demonstration of using docker to start a mysql simulation scenario.
    let mysql_container = TardisTestContainer::postgres_custom(None).await?;
    let port = mysql_container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:123456@localhost:{port}/test");
    env::set_var("TARDIS_FW.DB.URL", url);

    env::set_var("RUST_LOG", "debug");
    env::set_var("PROFILE", "default");

    // Initial configuration
    TardisFuns::init(Some("config")).await?;

    let db = TardisFuns::reldb().conn();

    db.execute_one(
        r###"CREATE TABLE graph
(
    node1 character varying NOT NULL,
    node2 character varying NOT NULL,
    kind character varying NOT NULL,
    reverse bool DEFAULT false NOT NULL, 
    ts timestamp without time zone DEFAULT CURRENT_TIMESTAMP NOT NULL,
    check (node1<>node2),  
    unique (node1,node2,kind)  
);"###,
        Vec::new(),
    )
    .await?;

    db.execute_one(r###"CREATE INDEX idx_node1 ON graph USING btree(node1);"###, Vec::new()).await?;
    db.execute_one(r###"CREATE INDEX idx_node2 ON graph USING btree(node2);"###, Vec::new()).await?;
    db.execute_one(r###"CREATE INDEX idx_kind ON graph USING btree(kind);"###, Vec::new()).await?;
    db.execute_one(r###"CREATE INDEX idx_ts ON graph USING btree(ts);"###, Vec::new()).await?;

    db.execute_one(
        r###"CREATE OR REPLACE FUNCTION GRAPH_SEARCH(
  IN I_ROOT CHARACTER varying,
  IN I_KIND CHARACTER varying DEFAULT '', IN I_LIMIT int8 DEFAULT 2000000000,
  IN I_DEPTH int DEFAULT 99999,
 OUT O_PATHS CHARACTER varying[],
 OUT O_NODE1 CHARACTER varying,
 OUT O_NODE2 CHARACTER varying,
 OUT O_KIND CHARACTER varying,
 OUT O_KINDS CHARACTER varying[],
 OUT O_DEPTH int,
 OUT O_REVERSE BOOL) RETURNS
SETOF RECORD AS $$
declare
  sql text;
begin
sql := format($_$
WITH RECURSIVE search_graph(
  node1,
  node2,
  kind,
  kinds,
  depth,
  paths,
	reverse
) AS (
        select node1,node2,kind,kinds,depth,paths,reverse from (
        SELECT
          g.node1,
          g.node2,
          g.kind as kind,
	      ARRAY[g.kind] as kinds,
          1 as depth,
          ARRAY[g.node1, g.node2] as paths,
        g.reverse
			FROM graph AS g
        WHERE
          node1 = '%s'
          limit %s
        ) t
      UNION ALL
        select node1,node2,kind,kinds,depth,paths,reverse from (
        SELECT
          DISTINCT ON (g.node1,g.node2,g.kind)
			g.node1,
          g.node2,
          g.kind as kind,
	      sg.kinds || g.kind as kinds,
          sg.depth + 1 as depth,
          sg.paths || g.node2 as paths,
          g.reverse
			FROM graph AS g, search_graph AS sg
        WHERE
          g.node1 = sg.node2
          AND g.node2 <> ALL(sg.paths)
          AND sg.depth <= %s
          limit %s
        ) t
)
SELECT paths as o_paths, node1 as o_node1, node2 as o_node2, kind as o_kind, kinds as o_kinds, depth as o_depth, reverse as o_reverse
FROM search_graph;
$_$, i_root, i_limit,i_depth,i_limit
);
return query execute sql;
end;
$$ LANGUAGE PLPGSQL STRICT;"###,
        Vec::new(),
    )
    .await?;

    db.execute_many(
        r###"INSERT INTO graph(node1, node2, kind, reverse) VALUES ($1, $2, $3, $4)"###,
        vec![
            vec!["req1".into(), "task1".into(), "req-task".into(), false.into()],
            vec!["task1".into(), "req1".into(), "req-task".into(), true.into()],
            vec!["req1".into(), "task2".into(), "req-task".into(), false.into()],
            vec!["task2".into(), "req1".into(), "req-task".into(), true.into()],
            vec!["task1".into(), "bug1".into(), "task-bug".into(), false.into()],
            vec!["bug1".into(), "task1".into(), "task-bug".into(), true.into()],
            vec!["task1".into(), "bug2".into(), "task-bug".into(), false.into()],
            vec!["bug2".into(), "task1".into(), "task-bug".into(), true.into()],
        ],
    )
    .await?;

    let result = db.query_all("SELECT * FROM GRAPH_SEARCH($1) ORDER BY O_DEPTH, O_KIND;", vec!["task1".into()]).await?;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0].try_get::<Vec<String>>("", "o_paths").unwrap(), vec!["task1", "req1"]);
    assert_eq!(result[0].try_get::<String>("", "o_node1").unwrap(), r#"task1"#);
    assert_eq!(result[0].try_get::<String>("", "o_node2").unwrap(), r#"req1"#);
    assert_eq!(result[0].try_get::<String>("", "o_kind").unwrap(), r#"req-task"#);
    assert_eq!(result[0].try_get::<Vec<String>>("", "o_kinds").unwrap(), vec![r#"req-task"#]);
    assert_eq!(result[0].try_get::<i32>("", "o_depth").unwrap(), 1);
    assert!(result[0].try_get::<bool>("", "o_reverse").unwrap());
    Ok(())
}
