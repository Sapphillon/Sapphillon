if (!globalThis.Sapphillon) {
  globalThis.Sapphillon = {};
}
if (!globalThis.Sapphillon.WebScraper) {
  globalThis.Sapphillon.WebScraper = {};
}

const ops = Deno.core.ops;

Object.assign(globalThis.Sapphillon.WebScraper, {
  create: () => ops.op2_ws_create(),
  destroy: (instanceId) => ops.op2_ws_destroy({ instanceId }),
  navigate: (instanceId, url) => ops.op2_ws_navigate({ instanceId, url }),
  getURI: (instanceId) => ops.op2_ws_get_uri({ instanceId }),
  getHTML: (instanceId) => ops.op2_ws_get_html({ instanceId }),
  getElement: (instanceId, selector) =>
    ops.op2_ws_get_element({ instanceId, selector }),
  getElementText: (instanceId, selector) =>
    ops.op2_ws_get_element_text({ instanceId, selector }),
  getValue: (instanceId, selector) =>
    ops.op2_ws_get_value({ instanceId, selector }),
  clickElement: (instanceId, selector) =>
    ops.op2_ws_click_element({ instanceId, selector }),
  waitForElement: (instanceId, selector, timeout) =>
    ops.op2_ws_wait_for_element({ instanceId, selector, timeout }),
  executeScript: (instanceId, script) =>
    ops.op2_ws_execute_script({ instanceId, script }),
  takeScreenshot: (instanceId) => ops.op2_ws_take_screenshot({ instanceId }),
  takeElementScreenshot: (instanceId, selector) =>
    ops.op2_ws_take_element_screenshot({ instanceId, selector }),
  takeFullPageScreenshot: (instanceId) =>
    ops.op2_ws_take_full_page_screenshot({ instanceId }),
  takeRegionScreenshot: (instanceId, rect) =>
    ops.op2_ws_take_region_screenshot({ instanceId, rect }),
  fillForm: (instanceId, formData) =>
    ops.op2_ws_fill_form({ instanceId, formData }),
  submit: (instanceId, selector) => ops.op2_ws_submit({ instanceId, selector }),
});
