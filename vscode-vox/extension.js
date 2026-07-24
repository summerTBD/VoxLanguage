const { LanguageClient } = require("vscode-languageclient/node");
const path = require("path");
const fs = require("fs");

/** @param {import("vscode").ExtensionContext} ctx */
function activate(ctx) {
    // 解析符号链接/ Junction 到真实路径
    const realPath = fs.realpathSync(ctx.extensionPath);
    const serverPath = path.join(
        realPath, "..", "target", "debug", "voxlsp"
    );

    const client = new LanguageClient(
        "vox-lsp",
        "Vox Language Server",
        { command: serverPath },
        { documentSelector: [{ language: "vox" }] }
    );

    ctx.subscriptions.push(client.start());
}

module.exports = { activate };
