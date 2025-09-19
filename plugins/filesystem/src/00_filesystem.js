function readFile(path) {
    return Deno.core.ops.op2_filesystem_read(path);
}

globalThis.fs.read = readFile;
