use deno_core::op2;
use sapphillon_core::plugin::{CorePluginPackage, CorePluginFunction};
use floorp_grpc::tab_manager::{
    tab_manager_service_client::TabManagerServiceClient as TabManagerClient,
    CreateInstanceRequest,
    CreateTabOptions,
    AttachToTabRequest,
    DestroyInstanceRequest,
    NavigateRequest,
    GetUriRequest,
    GetHtmlRequest,
    GetElementRequest,
    GetElementTextRequest,
    GetValueRequest,
    ClickElementRequest,
    WaitForElementRequest,
    ExecuteScriptRequest,
    TakeScreenshotRequest,
    ScreenshotType,
    ScreenshotRect,
    FillFormRequest,
    SubmitFormRequest,
    ListTabsRequest,
    GetInstanceInfoRequest,
    DetailedTabInfo,
};
use std::collections::HashMap;
use std::sync::{OnceLock, Mutex};
use serde::Serialize;
use deno_error::JsErrorBox;

const GRPC_SERVER_ADDRESS: &str = "http://[::1]:50051";

static TAB_MANAGER_CLIENT: OnceLock<Mutex<TabManagerClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, TabManagerClient<tonic::transport::Channel>>> {
    let lock = TAB_MANAGER_CLIENT.get_or_init(|| {
        let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9").connect_lazy();
        Mutex::new(TabManagerClient::new(ch))
    });
    lock.lock().ok()
}

macro_rules! plugin_function {
    ($name:ident, $op:expr, $js_name:expr, $desc:expr) => {
        CorePluginFunction::new(
            format!("app.floorp.tab_manager.{}", stringify!($name)),
            $js_name.to_string(),
            $desc.to_string(),
            $op,
            None,
        )
    };
    ($name:ident, $op:expr, $js_name:expr, $desc:expr, with_js: $js:expr) => {
        CorePluginFunction::new(
            format!("app.floorp.tab_manager.{}", stringify!($name)),
            $js_name.to_string(),
            $desc.to_string(),
            $op,
            Some($js.to_string()),
        )
    };
}

pub fn tab_manager_plugin_package() -> CorePluginPackage {
    CorePluginPackage::new(
        "app.floorp.tab_manager".into(),
        "Floorp Tab Manager".into(),
        vec![
            plugin_function!(tm_create_instance, op2_tm_create_instance(), "tmCreateInstance", "Open a new visible tab", with_js: include_str!("00_tab_manager.js")),
            plugin_function!(tm_attach_to_tab, op2_tm_attach_to_tab(), "tmAttachToTab", "Attach to an existing visible tab"),
            plugin_function!(list_tabs, op2_list_tabs(), "tmListTabs", "List all visible tabs"),
            plugin_function!(get_instance_info, op2_get_instance_info(), "tmGetInstanceInfo", "Get aggregated instance information"),
            plugin_function!(tm_destroy_instance, op2_tm_destroy_instance(), "tmDestroyInstance", "Close the tab associated with the instance"),
            plugin_function!(tm_navigate, op2_tm_navigate(), "tmNavigate", "Navigate the visible tab to a URL"),
            plugin_function!(tm_get_uri, op2_tm_get_uri(), "tmGetURI", "Get current URI of the visible tab"),
            plugin_function!(tm_get_html, op2_tm_get_html(), "tmGetHTML", "Get full document HTML"),
            plugin_function!(tm_get_element, op2_tm_get_element(), "tmGetElement", "Get an element's outerHTML by selector"),
            plugin_function!(tm_get_element_text, op2_tm_get_element_text(), "tmGetElementText", "Get an element's textContent by selector"),
            plugin_function!(tm_get_value, op2_tm_get_value(), "tmGetValue", "Get an input/textarea value by selector"),
            plugin_function!(tm_click_element, op2_tm_click_element(), "tmClickElement", "Click an element by selector"),
            plugin_function!(tm_wait_for_element, op2_tm_wait_for_element(), "tmWaitForElement", "Wait for an element to appear"),
            plugin_function!(tm_execute_script, op2_tm_execute_script(), "tmExecuteScript", "Execute a script in the page context"),
            plugin_function!(tm_take_screenshot, op2_tm_take_screenshot(), "tmTakeScreenshot", "Take a viewport screenshot"),
            plugin_function!(tm_take_element_screenshot, op2_tm_take_element_screenshot(), "tmTakeElementScreenshot", "Take a screenshot of a specific element"),
            plugin_function!(tm_take_full_page_screenshot, op2_tm_take_full_page_screenshot(), "tmTakeFullPageScreenshot", "Take a full page screenshot"),
            plugin_function!(tm_take_region_screenshot, op2_tm_take_region_screenshot(), "tmTakeRegionScreenshot", "Take a screenshot of a specific region"),
            plugin_function!(tm_fill_form, op2_tm_fill_form(), "tmFillForm", "Fill a form with data"),
            plugin_function!(tm_submit, op2_tm_submit(), "tmSubmit", "Submit a form"),
            plugin_function!(tm_wait, op2_tm_wait(), "tmWait", "Sleep helper"),
        ]
    )
}

// Common structs for parameters
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceIdParam { instance_id: String }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateInstanceParam { url: String, #[serde(default)] options: Option<CreateTabOptionsParam> }

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CreateTabOptionsParam { in_background: Option<bool> }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachToTabParam { browser_id: String }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectorParam { instance_id: String, selector: String }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NavigateParam { instance_id: String, url: String }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WaitForElementParam { instance_id: String, selector: String, #[serde(default)] timeout: Option<u32> }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteScriptParam { instance_id: String, script: String }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegionScreenshotParam { instance_id: String, #[serde(default)] rect: Option<RectParam> }

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RectParam { x: Option<f32>, y: Option<f32>, width: Option<f32>, height: Option<f32> }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FillFormParam { instance_id: String, form_data: HashMap<String, String> }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WaitParam { ms: u32 }

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct TabView<'a>{
    instance_id: &'a str,
    browser_id: &'a str,
    uri: &'a str,
    title: &'a str,
    is_active: bool
}

// Op Functions
#[op2(async)]
#[string]
pub async fn op2_tm_create_instance(#[serde] params: CreateInstanceParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let options = params.options.unwrap_or_default();
    let req = CreateInstanceRequest {
        url: params.url,
        options: Some(CreateTabOptions { in_background: options.in_background }),
    };
    let resp = guard.create_instance(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().instance_id)
}

#[op2(async)]
#[string]
pub async fn op2_tm_attach_to_tab(#[serde] params: AttachToTabParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = AttachToTabRequest { browser_id: params.browser_id };
    let resp = guard.attach_to_tab(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().instance_id.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_list_tabs() -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
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

#[op2(async)]
#[string]
pub async fn op2_get_instance_info(#[serde] param: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetInstanceInfoRequest{ instance_id: param.instance_id };
    let resp = guard.get_instance_info(req).await
        .map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
        .into_inner();
    Ok(resp.instance_info.unwrap_or_else(||"{}".into()))
}

#[op2(async)]
pub async fn op2_tm_destroy_instance(#[serde] params: InstanceIdParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    guard.destroy_instance(DestroyInstanceRequest { instance_id: params.instance_id }).await
        .map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
pub async fn op2_tm_navigate(#[serde] params: NavigateParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = NavigateRequest { instance_id: params.instance_id, url: params.url };
    guard.navigate(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
#[string]
pub async fn op2_tm_get_uri(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetUriRequest { instance_id: params.instance_id };
    let resp = guard.get_uri(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().uri)
}

#[op2(async)]
#[string]
pub async fn op2_tm_get_html(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetHtmlRequest { instance_id: params.instance_id };
    let resp = guard.get_html(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().html.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_get_element(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetElementRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().element_html.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_get_element_text(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetElementTextRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_element_text(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().text.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_get_value(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetValueRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_value(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().value.unwrap_or_default())
}

#[op2(async)]
pub async fn op2_tm_click_element(#[serde] params: SelectorParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = ClickElementRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.click_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}

#[op2(async)]
pub async fn op2_tm_wait_for_element(#[serde] params: WaitForElementParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = WaitForElementRequest { instance_id: params.instance_id, selector: params.selector, timeout: params.timeout.map(|t| t as i32) };
    let resp = guard.wait_for_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().found)
}

#[op2(async)]
pub async fn op2_tm_execute_script(#[serde] params: ExecuteScriptParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = ExecuteScriptRequest { instance_id: params.instance_id, script: params.script };
    guard.execute_script(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
#[string]
pub async fn op2_tm_take_screenshot(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::Viewport as i32, selector: None, rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_take_element_screenshot(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::Element as i32, selector: Some(params.selector), rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_take_full_page_screenshot(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::FullPage as i32, selector: None, rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_tm_take_region_screenshot(#[serde] params: RegionScreenshotParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let rect = params.rect.unwrap_or_default();
    let req = TakeScreenshotRequest {
        instance_id: params.instance_id,
        r#type: ScreenshotType::Region as i32,
        selector: None,
        rect: Some(ScreenshotRect {
            x: rect.x.map(|v| v as i32),
            y: rect.y.map(|v| v as i32),
            width: rect.width.map(|v| v as i32),
            height: rect.height.map(|v| v as i32),
        })
    };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
pub async fn op2_tm_fill_form(#[serde] params: FillFormParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = FillFormRequest { instance_id: params.instance_id, form_data: params.form_data };
    let resp = guard.fill_form(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}

#[op2(async)]
pub async fn op2_tm_submit(#[serde] params: SelectorParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = TabManagerClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = SubmitFormRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.submit_form(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}

#[op2(async)]
pub async fn op2_tm_wait(#[serde] params: WaitParam) -> Result<(), JsErrorBox> {
    // Local sleep (Wait RPC removed from proto)
    tokio::time::sleep(std::time::Duration::from_millis(params.ms as u64)).await;
    Ok(())
}
