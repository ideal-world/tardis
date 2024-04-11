use criterion::{criterion_group, criterion_main, Criterion};

use tardis::serde::{Deserialize, Serialize};
use tardis::TardisFuns;

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct TestConfig<T> {
    project_name: String,
    level_num: u8,
    db_proj: T,
}

impl<T: Default> Default for TestConfig<T> {
    fn default() -> Self {
        TestConfig {
            project_name: String::new(),
            level_num: 0,
            db_proj: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct DatabaseConfig {
    url: String,
}

pub fn json_process(c: &mut Criterion) {
    let test_config = TestConfig {
        project_name: "测试".to_string(),
        level_num: 0,
        db_proj: DatabaseConfig { url: "http://xxx".to_string() },
    };
    c.bench_function("JSON: obj_to_string", |b| {
        b.iter(|| {
            TardisFuns::json.obj_to_string(&test_config).unwrap();
        })
    });
    let json_str = r#"{"project_name":"测试","level_num":0,"db_proj":{"url":"http://xxx"}}"#.to_string();
    c.bench_function("JSON: str_to_obj", |b| {
        b.iter(|| {
            TardisFuns::json.str_to_obj::<TestConfig<DatabaseConfig>>(&json_str).unwrap();
        })
    });
}

criterion_group!(benches, json_process);
criterion_main!(benches);
