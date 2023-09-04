use tardis::basic::result::TardisResult;
use tardis::web::web_server::ArcTardisWebServer;

pub async fn init(web_server: &ArcTardisWebServer) -> TardisResult<()> {
    tardis_example_multi_apps_doc::initializer::init(web_server).await?;
    tardis_example_multi_apps_tag::initializer::init(web_server).await?;
    Ok(())
}
