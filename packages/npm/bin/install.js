const fs = require("fs");
const path = require("path");
const https = require("https");

const platform = process.platform;
const arch = process.arch;

let assetName = "";

if (platform === "linux" && arch === "x64") {
  assetName = "graft-linux-x86_64";
} else if (platform === "darwin" && arch === "arm64") {
  assetName = "graft-macos-arm64";
} else if (platform === "win32" && arch === "x64") {
  assetName = "graft-windows-x86_64.exe";
} else {
  process.exit(0);
}

const targetBinaryName = platform === "win32" ? "graft.exe" : "graft";
const destinationPath = path.join(__dirname, targetBinaryName);

const releaseUrl = `https://github.com/idunnowutuwant/graft/releases/latest/download/${assetName}`;

function download(url, dest, cb) {
  const file = fs.createWriteStream(dest);
  https.get(url, (response) => {
    if (response.statusCode === 302 || response.statusCode === 301) {
      download(response.headers.location, dest, cb);
      return;
    }
    if (response.statusCode !== 200) {
      fs.unlink(dest, () => {});
      return;
    }
    response.pipe(file);
    file.on("finish", () => {
      file.close(() => {
        if (platform !== "win32") {
          fs.chmodSync(dest, 0o755);
        }
        if (cb) cb();
      });
    });
  }).on("error", () => {
    fs.unlink(dest, () => {});
  });
}

download(releaseUrl, destinationPath);