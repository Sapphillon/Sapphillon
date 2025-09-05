use deno_core::op2;
use sapphillon_core::plugin::{CorePluginPackage, CorePluginFunction};
use floorp_grpc::webscraper::{
	tab_manager_service_client::TabManagerServiceClient as WebscraperClient,
	CreateInstanceRequest, CreateTabOptions,
	GetHtmlRequest
};
use std::sync::{OnceLock, Mutex};
use deno_error::JsErrorBox;

const GRPC_SERVER_ADDRESS: &str = "http://[::1]:50051";

static WEBSCRAPER_CLIENT: OnceLock<Mutex<WebscraperClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, WebscraperClient<tonic::transport::Channel>>> {
	let lock = WEBSCRAPER_CLIENT.get_or_init(|| {
		let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9").connect_lazy();
		Mutex::new(WebscraperClient::new(ch))
	});
	lock.lock().ok()
}

pub fn webscraper_plugin_package() -> CorePluginPackage {
	CorePluginPackage::new(
		"app.floorp.webscraper".into(),
		"Floorp Webscraper".into(),
		vec![create_instance_plugin(), get_html_plugin()]
	)
}

fn create_instance_plugin() -> CorePluginFunction {
	CorePluginFunction::new(
		"app.floorp.webscraper.create_instance".into(),
		"CreateInstance".into(),
		"Create a new scraping instance".into(),
		op2_create_instance(),
		Some(include_str!("00_webscraper.js").to_string())
	)
}

fn get_html_plugin() -> CorePluginFunction {
	CorePluginFunction::new(
		"app.floorp.webscraper.get_html".into(),
		"GetHTML".into(),
		"Get document HTML for instance".into(),
		op2_get_html(),
		None
	)
}

#[derive(serde::Deserialize)]
#[serde(rename_all="camelCase")]
struct CreateParams { url: String, #[serde(default)] in_background: Option<bool> }

#[op2(async)]
#[string]
pub async fn op2_create_instance(#[serde] params: CreateParams) -> Result<String, JsErrorBox> {
	let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
	if let Err(e) = WebscraperClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
		return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
	}
	let req = CreateInstanceRequest { url: params.url, options: Some(CreateTabOptions{ in_background: params.in_background }) };
	let resp = guard.create_instance(req).await
		.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
		.into_inner();
	Ok(resp.instance_id)
}

#[derive(serde::Deserialize)]
struct InstanceId { instance_id: String }

#[op2(async)]
#[string]
pub async fn op2_get_html(#[serde] instance: InstanceId) -> Result<String, JsErrorBox> {
	let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
	if let Err(e) = WebscraperClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
		return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
	}
	let req = GetHtmlRequest { instance_id: instance.instance_id };
	let resp = guard.get_html(req).await
		.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
		.into_inner();
	Ok(resp.html.unwrap_or_else(||"".into()))
}
