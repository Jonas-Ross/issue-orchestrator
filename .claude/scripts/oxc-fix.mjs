#!/usr/bin/env node
// PostToolUse hook: auto-fix oxlint + oxfmt on TS/TSX files the agent
// just touched. Best-effort — exits 0 even on tool failures so a
// transient lint error doesn't break the agent loop. CI is the gate.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";

const CWD = process.cwd();
const BINDINGS = "src/lib/bindings.ts";

function readEvent() {
  try {
    return JSON.parse(readFileSync(0, "utf8"));
  } catch {
    return null;
  }
}

function collectPaths(event) {
  const input = event?.tool_input;
  if (!input) return [];
  const paths = [];
  if (typeof input.file_path === "string") paths.push(input.file_path);
  if (Array.isArray(input.edits)) {
    for (const e of input.edits) {
      if (typeof e?.file_path === "string") paths.push(e.file_path);
    }
  }
  return paths;
}

function inScope(p) {
  const abs = isAbsolute(p) ? p : resolve(CWD, p);
  const rel = relative(CWD, abs);
  if (rel.startsWith("..") || isAbsolute(rel)) return false;
  if (!/\.(ts|tsx)$/.test(rel)) return false;
  if (!rel.startsWith("src/")) return false;
  if (rel === BINDINGS) return false;
  if (!existsSync(abs)) return false;
  return rel;
}

function run(cmd, args) {
  try {
    execFileSync(cmd, args, { cwd: CWD, stdio: ["ignore", "pipe", "pipe"] });
  } catch (err) {
    // Forward stderr so the agent sees what couldn't be auto-fixed.
    if (err?.stderr) process.stderr.write(err.stderr);
    if (err?.stdout) process.stderr.write(err.stdout);
  }
}

const event = readEvent();
if (!event) process.exit(0);

const targets = collectPaths(event)
  .map(inScope)
  .filter((rel) => rel !== false);

if (targets.length === 0) process.exit(0);

const unique = [...new Set(targets)];
run("npx", ["--no-install", "oxlint", "--fix", ...unique]);
run("npx", ["--no-install", "oxfmt", ...unique]);
process.exit(0);
