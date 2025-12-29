((globalThis) => {
    const { core } = Deno;
    const { ops } = core;

    const PACKAGE_ID = "app.sapphillon.core.window";

    function get_active_window_title() {
        return ops.op2_get_active_window_title();
    }

    function get_inactive_window_titles() {
        return ops.op2_get_inactive_window_titles();
    }

    const functions = {
        get_active_window_title,
        get_inactive_window_titles,
    };

    let a = PACKAGE_ID.split("."), o = globalThis;
    for (let i = 0; i < a.length; i++) {
        let p = a[i];
        if (o[p] === undefined) {
            o[p] = {};
        }
        if (i === a.length - 1) {
            if (Object.keys(o[p]).length > 0) {
                throw new Error(`Sapphillon plugin package already loaded: ${PACKAGE_ID}`);
            }
            o[p] = functions;
        }
        o = o[p];
    }
})(globalThis);
