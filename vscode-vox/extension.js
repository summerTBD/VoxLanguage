const { LanguageClient } = require("vscode-languageclient/node");
const path = require("path");
const fs = require("fs");

/** @param {import("vscode").ExtensionContext} ctx */
function activate(ctx) {
    const realDir = fs.realpathSync(ctx.extensionPath);
    const exe = process.platform === "win32" ? "voxlsp.exe" : "voxlsp";

    // 优先级：debug（开发时无锁） > release > PATH
    const candidates = [
        path.join(realDir, "..", "target", "debug", exe),
        path.join(realDir, "..", "target", "release", exe),
        exe,
    ];
    const serverPath = candidates.find(p => fs.existsSync(p)) || candidates[2];

    console.log(`Vox LSP: ${serverPath}`);

    const client = new LanguageClient(
        "vox-lsp", "Vox Language Server",
        { command: serverPath },
        { documentSelector: [{ language: "vox" }] }
    );
    ctx.subscriptions.push(client.start());
}

module.exports = { activate };
