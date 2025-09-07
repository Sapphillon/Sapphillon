if (!globalThis.Sapphillon) {
  globalThis.Sapphillon = {};
}
if (!globalThis.Sapphillon.TabManager) {
  globalThis.Sapphillon.TabManager = {};
}

const ops = Deno.core.ops;

Object.assign(globalThis.Sapphillon.TabManager, {
  createInstance: (url, options) =>
    ops.op2_tm_create_instance({ url, options }),
  attachToTab: (browserId) => ops.op2_tm_attach_to_tab({ browserId }),
  listTabs: () => ops.op2_list_tabs(),
  getInstanceInfo: (instanceId) => ops.op2_get_instance_info({ instanceId }),
  destroyInstance: (instanceId) => ops.op2_tm_destroy_instance({ instanceId }),
  navigate: (instanceId, url) => ops.op2_tm_navigate({ instanceId, url }),
  getURI: (instanceId) => ops.op2_tm_get_uri({ instanceId }),
  getHTML: (instanceId) => ops.op2_tm_get_html({ instanceId }),
  getElement: (instanceId, selector) =>
    ops.op2_tm_get_element({ instanceId, selector }),
  getElementText: (instanceId, selector) =>
    ops.op2_tm_get_element_text({ instanceId, selector }),
  getValue: (instanceId, selector) =>
    ops.op2_tm_get_value({ instanceId, selector }),
  clickElement: (instanceId, selector) =>
    ops.op2_tm_click_element({ instanceId, selector }),
  waitForElement: (instanceId, selector, timeout) =>
    ops.op2_tm_wait_for_element({ instanceId, selector, timeout }),
  executeScript: (instanceId, script) =>
    ops.op2_tm_execute_script({ instanceId, script }),
  takeScreenshot: (instanceId) => ops.op2_tm_take_screenshot({ instanceId }),
  takeElementScreenshot: (instanceId, selector) =>
    ops.op2_tm_take_element_screenshot({ instanceId, selector }),
  takeFullPageScreenshot: (instanceId) =>
    ops.op2_tm_take_full_page_screenshot({ instanceId }),
  takeRegionScreenshot: (instanceId, rect) =>
    ops.op2_tm_take_region_screenshot({ instanceId, rect }),
  fillForm: (instanceId, formData) =>
    ops.op2_tm_fill_form({ instanceId, formData }),
  submit: (instanceId, selector) => ops.op2_tm_submit({ instanceId, selector }),
  wait: (ms) => ops.op2_tm_wait({ ms }),
});
