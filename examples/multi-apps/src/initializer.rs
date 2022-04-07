use tardis::basic::result::TardisResult;
use tardis::web::web_server::TardisWebServer;

pub async fn init(web_server: &TardisWebServer) -> TardisResult<()> {
    tardis_example_multi_apps_doc::initializer::init(web_server).await?;
    tardis_example_multi_apps_tag::initializer::init(web_server).await?;
    Ok(())
}
