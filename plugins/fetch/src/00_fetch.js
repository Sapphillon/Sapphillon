((globalThis) => {
    const { core } = Deno;
    const { ops } = core;

    const PACKAGE_ID = "app.sapphillon.core.fetch";

    function fetch(url) {
        return ops.op2_fetch(url);
    }

    function post(url, body) {
        return ops.op2_post(url, body);
    }

    const functions = {
        fetch,
        post,
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
