const vscode = require("vscode");
const { exec } = require("child_process");

function activate(context) {
  let statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  statusBar.text = "$(git-merge) Graft: Ready";
  statusBar.tooltip = "Click to run Graft Proactive Conflict Radar";
  statusBar.command = "graft.radar";
  statusBar.show();
  context.subscriptions.push(statusBar);

  context.subscriptions.push(
    vscode.commands.registerCommand("graft.radar", () => {
      exec("graft radar main", (err, stdout) => {
        if (err) {
          vscode.window.showErrorMessage(`Graft Radar Error: ${err.message}`);
          return;
        }
        vscode.window.showInformationMessage(stdout);
      });
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("graft.undo", () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const file = editor.document.fileName;
      exec(`graft undo "${file}"`, (err, stdout) => {
        if (err) {
          vscode.window.showErrorMessage(`Rollback failed: ${err.message}`);
        } else {
          vscode.window.showInformationMessage(stdout);
        }
      });
    })
  );
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};