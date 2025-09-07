use deno_core::op2;
use sapphillon_core::plugin::{CorePluginPackage, CorePluginFunction};
use floorp_grpc::webscraper::{
    tab_manager_service_client::TabManagerServiceClient as WebScraperServiceClient,
    CreateInstanceRequest,
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
};
use std::collections::HashMap;
use std::sync::{OnceLock, Mutex};
use deno_error::JsErrorBox;

const GRPC_SERVER_ADDRESS: &str = "http://[::1]:50051";

static WEBSCRAPER_CLIENT: OnceLock<Mutex<WebScraperServiceClient<tonic::transport::Channel>>> = OnceLock::new();

fn get_or_init_client() -> Option<std::sync::MutexGuard<'static, WebScraperServiceClient<tonic::transport::Channel>>> {
    let lock = WEBSCRAPER_CLIENT.get_or_init(|| {
        let ch = tonic::transport::Endpoint::from_static("http://127.0.0.1:9").connect_lazy();
        Mutex::new(WebScraperServiceClient::new(ch))
    });
    lock.lock().ok()
}

macro_rules! plugin_function {
    ($name:ident, $op:expr, $js_name:expr, $desc:expr) => {
        CorePluginFunction::new(
            format!("app.floorp.webscraper.{}", stringify!($name)),
            $js_name.to_string(),
            $desc.to_string(),
            $op,
            None, // JS file is loaded once in the first function
        )
    };
    ($name:ident, $op:expr, $js_name:expr, $desc:expr, with_js: $js:expr) => {
        CorePluginFunction::new(
            format!("app.floorp.webscraper.{}", stringify!($name)),
            $js_name.to_string(),
            $desc.to_string(),
            $op,
            Some($js.to_string()),
        )
    };
}

pub fn webscraper_plugin_package() -> CorePluginPackage {
    CorePluginPackage::new(
        "app.floorp.webscraper".into(),
        "Floorp Webscraper".into(),
        vec![
            plugin_function!(ws_create, op2_ws_create(), "wsCreate", "Create a new HiddenFrame instance", with_js: include_str!("00_webscraper.js")),
            plugin_function!(ws_destroy, op2_ws_destroy(), "wsDestroy", "Destroy an existing HiddenFrame instance"),
            plugin_function!(ws_navigate, op2_ws_navigate(), "wsNavigate", "Navigate the HiddenFrame to a URL"),
            plugin_function!(ws_get_uri, op2_ws_get_uri(), "wsGetURI", "Get current URI of the HiddenFrame"),
            plugin_function!(ws_get_html, op2_ws_get_html(), "wsGetHTML", "Get full document HTML"),
            plugin_function!(ws_get_element, op2_ws_get_element(), "wsGetElement", "Get an element's outerHTML by selector"),
            plugin_function!(ws_get_element_text, op2_ws_get_element_text(), "wsGetElementText", "Get an element's textContent by selector"),
            plugin_function!(ws_get_value, op2_ws_get_value(), "wsGetValue", "Get an input/textarea value by selector"),
            plugin_function!(ws_click_element, op2_ws_click_element(), "wsClickElement", "Click an element by selector"),
            plugin_function!(ws_wait_for_element, op2_ws_wait_for_element(), "wsWaitForElement", "Wait for an element to appear"),
            plugin_function!(ws_execute_script, op2_ws_execute_script(), "wsExecuteScript", "Execute a script in the page context"),
            plugin_function!(ws_take_screenshot, op2_ws_take_screenshot(), "wsTakeScreenshot", "Take a viewport screenshot"),
            plugin_function!(ws_take_element_screenshot, op2_ws_take_element_screenshot(), "wsTakeElementScreenshot", "Take a screenshot of a specific element"),
            plugin_function!(ws_take_full_page_screenshot, op2_ws_take_full_page_screenshot(), "wsTakeFullPageScreenshot", "Take a full page screenshot"),
            plugin_function!(ws_take_region_screenshot, op2_ws_take_region_screenshot(), "wsTakeRegionScreenshot", "Take a screenshot of a specific region"),
            plugin_function!(ws_fill_form, op2_ws_fill_form(), "wsFillForm", "Fill a form with data"),
            plugin_function!(ws_submit, op2_ws_submit(), "wsSubmit", "Submit a form"),
        ]
    )
}

// Common structs for parameters
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstanceIdParam { instance_id: String }

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


// Op Functions
#[op2(async)]
#[string]
pub async fn op2_ws_create() -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let resp = guard.create_instance(CreateInstanceRequest { url: String::new(), options: None }).await
        .map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?
        .into_inner();
    Ok(resp.instance_id)
}

#[op2(async)]
pub async fn op2_ws_destroy(#[serde] params: InstanceIdParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    guard.destroy_instance(DestroyInstanceRequest { instance_id: params.instance_id }).await
        .map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
pub async fn op2_ws_navigate(#[serde] params: NavigateParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = NavigateRequest { instance_id: params.instance_id, url: params.url };
    guard.navigate(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
#[string]
pub async fn op2_ws_get_uri(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetUriRequest { instance_id: params.instance_id };
    let resp = guard.get_uri(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().uri)
}

#[op2(async)]
#[string]
pub async fn op2_ws_get_html(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetHtmlRequest { instance_id: params.instance_id };
    let resp = guard.get_html(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().html.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_get_element(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetElementRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().element_html.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_get_element_text(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetElementTextRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_element_text(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().text.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_get_value(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = GetValueRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.get_value(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().value.unwrap_or_default())
}

#[op2(async)]
pub async fn op2_ws_click_element(#[serde] params: SelectorParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = ClickElementRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.click_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}

#[op2(async)]
pub async fn op2_ws_wait_for_element(#[serde] params: WaitForElementParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = WaitForElementRequest { instance_id: params.instance_id, selector: params.selector, timeout: params.timeout.map(|t| t as i32) };
    let resp = guard.wait_for_element(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().found)
}

#[op2(async)]
pub async fn op2_ws_execute_script(#[serde] params: ExecuteScriptParam) -> Result<(), JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = ExecuteScriptRequest { instance_id: params.instance_id, script: params.script };
    guard.execute_script(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(())
}

#[op2(async)]
#[string]
pub async fn op2_ws_take_screenshot(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::Viewport as i32, selector: None, rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_take_element_screenshot(#[serde] params: SelectorParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::Element as i32, selector: Some(params.selector), rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_take_full_page_screenshot(#[serde] params: InstanceIdParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = TakeScreenshotRequest { instance_id: params.instance_id, r#type: ScreenshotType::FullPage as i32, selector: None, rect: None };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
#[string]
pub async fn op2_ws_take_region_screenshot(#[serde] params: RegionScreenshotParam) -> Result<String, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let rect = params.rect.unwrap_or_default();
    let req = TakeScreenshotRequest {
        instance_id: params.instance_id,
        r#type: ScreenshotType::Region as i32,
        selector: None,
        rect: Some(ScreenshotRect { x: rect.x.map(|v| v as i32), y: rect.y.map(|v| v as i32), width: rect.width.map(|v| v as i32), height: rect.height.map(|v| v as i32) })
    };
    let resp = guard.take_screenshot(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().screenshot_data.unwrap_or_default())
}

#[op2(async)]
pub async fn op2_ws_fill_form(#[serde] params: FillFormParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = FillFormRequest { instance_id: params.instance_id, form_data: params.form_data };
    let resp = guard.fill_form(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}

#[op2(async)]
pub async fn op2_ws_submit(#[serde] params: SelectorParam) -> Result<bool, JsErrorBox> {
    let mut guard = get_or_init_client().ok_or_else(|| JsErrorBox::new("Error", "Mutex poisoned"))?;
    if let Err(e) = WebScraperServiceClient::connect(GRPC_SERVER_ADDRESS).await.map(|c| { *guard = c; }) {
        return Err(JsErrorBox::new("Error", format!("gRPC connection failed: {e}")));
    }
    let req = SubmitFormRequest { instance_id: params.instance_id, selector: params.selector };
    let resp = guard.submit_form(req).await.map_err(|e| JsErrorBox::new("Error", format!("RPC failed: {e}")))?;
    Ok(resp.into_inner().success)
}
