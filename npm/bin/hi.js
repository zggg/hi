#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const MAP = {
  "darwin-arm64": "hi-darwin-arm64",
  "darwin-x64": "hi-darwin-x64",
  "linux-x64": "hi-linux-x64",
  "linux-arm64": "hi-linux-arm64",
};

const key = `${process.platform}-${process.arch}`;
const name = MAP[key];
if (!name) {
  console.error(`hi 不支持 ${key}`);
  process.exit(1);
}

const bin = path.join(__dirname, "..", "dist", name);
if (!fs.existsSync(bin)) {
  console.error(`缺少 dist/${name}，请先运行 ./scripts/build-dist.sh`);
  process.exit(1);
}

const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(r.status ?? 1);
