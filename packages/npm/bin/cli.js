#!/usr/bin/env node
const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binName = process.platform === "win32" ? "graft.exe" : "graft";
const localBin = path.join(__dirname, binName);

if (!fs.existsSync(localBin)) {
  const result = spawnSync("graft", process.argv.slice(2), { stdio: "inherit" });
  process.exit(result.status || 0);
} else {
  const result = spawnSync(localBin, process.argv.slice(2), { stdio: "inherit" });
  process.exit(result.status || 0);
}