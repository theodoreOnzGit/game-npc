#!/usr/bin/env -S node --no-warnings
// SPDX-License-Identifier: AGPL-3.0-only
// Cargo runner for wasm32-wasip1 test binaries using Node's built-in WASI.
// Usage (set in .cargo/config.toml): scripts/run-wasi.mjs <file.wasm> [args...]
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";
import process from "node:process";

const [wasmPath, ...args] = process.argv.slice(2);
const wasi = new WASI({ version: "preview1", args: [wasmPath, ...args], env: {}, returnOnExit: true });
const module = await WebAssembly.compile(await readFile(wasmPath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
