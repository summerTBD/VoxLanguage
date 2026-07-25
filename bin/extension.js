const { LanguageClient } = require("vscode-languageclient/node");
const path = require("path");

function activate(ctx) {
    const server = path.join(__dirname, "voxlsp.exe");
    const client = new LanguageClient(
        "vox-lsp", "Vox Language Server",
        { command: server },
        { documentSelector: [{ language: "vox" }] }
    );
    ctx.subscriptions.push(client.start());
}
module.exports = { activate };
