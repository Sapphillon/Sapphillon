use deno_core::op2;
use sapphillon_core::plugin::{CorePluginPackage, CorePluginFunction};
use floorp_grpc::tab_manager::{
	tab_manager_service_client::TabManagerServiceClient,
	ListTabsRequest, GetInstanceInfoRequest, DetailedTabInfo
};
use std::sync::{OnceLock, Mutex};
use serde::Serialize;
use deno_error::JsErrorBox;

const GRPC_SERVER_ADDRESS: &str = "http://[::1]:50051";

static TAB_MANAGER_CLIENT: OnceLock<Mutex<TabManagerServiceClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, TabManagerServiceClient<tonic::transport::Channel>>> {
	let lock = TAB_MANAGER_CLIENT.get_or_init(|| {
		let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9").connect_lazy();
		Mutex::new(TabManagerServiceClient::new(ch))
	});
	lock.lock().ok()
}

pub fn tab_manager_plugin_package() -> CorePluginPackage {
	CorePluginPackage::new(
		"app.floorp.tab_manager".into(),
		"Floorp Tab Manager".into(),
		vec![list_tabs_plugin(), get_instance_info_plugin()]
	)
}

fn list_tabs_plugin() -> CorePluginFunction {
	CorePluginFunction::new(
		"app.floorp.tab_manager.list_tabs".into(),
		"ListTabs".into(),
		"List all visible tabs".into(),
		op2_list_tabs(),
		Some(include_str!("00_tab_manager.js").to_string())
	)
}

fn get_instance_info_plugin() -> CorePluginFunction {
	CorePluginFunction::new(
		"app.floorp.tab_manager.get_instance_info".into(),
		"GetInstanceInfo".into(),
		"Get aggregated instance information".into(),
		op2_get_instance_info(),
		None
	)
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct TabView<'a>{
	instance_id: &'a str,
	browser_id: &'a str,
	uri: &'a str,
	title: &'a str,
	is_active: bool
}

#[op2(async)]
#[string]
pub async fn op2_list_tabs() -> Result<String, JsErrorBox> {
	let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
	if let Err(e) = TabManagerServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
		return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
	}
	let resp = guard.list_tabs(ListTabsRequest{}).await
		.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
		.into_inner();
	let tabs: Vec<TabView> = resp.tabs.iter().map(|t: &DetailedTabInfo| TabView {
		instance_id: t.instance_id.as_str(),
		browser_id: t.browser_id.as_str(),
		uri: t.uri.as_str(),
		title: t.title.as_str(),
		is_active: t.is_active,
	}).collect();
	Ok(serde_json::to_string(&tabs).unwrap())
}

#[derive(serde::Deserialize)]
struct InstanceIdParam { instance_id: String }

#[op2(async)]
#[string]
pub async fn op2_get_instance_info(#[serde] param: InstanceIdParam) -> Result<String, JsErrorBox> {
	let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
	if let Err(e) = TabManagerServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
		return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
	}
	let req = GetInstanceInfoRequest{ instance_id: param.instance_id };
	let resp = guard.get_instance_info(req).await
		.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
		.into_inner();
	Ok(resp.instance_info.unwrap_or_else(||"{}".into()))
}
