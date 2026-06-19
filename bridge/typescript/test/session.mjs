import { mkdtempSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import * as wasm from "@destack/language-wasm";

import {
    Edit,
    Source,
    openRepository as openPublicRepository,
    openWorkspace as openPublicWorkspace,
} from "../dist/index.js";
import { openNapiRepository, openNapiWorkspace } from "../dist/napi.js";
import { openWasmRepository, openWasmWorkspace } from "../dist/wasm.js";

export class RepositoryFixture {
    constructor(repository, root) {
        this.repository = repository;
        this.root = root;
    }
}

export class WorkspaceFixture {
    constructor(workspace, root) {
        this.workspace = workspace;
        this.root = root;
    }

    relativeUri(uri) {
        if (uri.startsWith(this.root)) {
            return uri.slice(this.root.length).replace(/^\//u, "");
        }

        return uri;
    }
}

export async function openWorkspace(open, files) {
    const root = testRoot();
    const edits = files.map(([path, text]) => Edit.setText(path, text));
    const source = Source.memory(root, edits);

    return new WorkspaceFixture(await open(source), root);
}

export async function openRepository(open, files) {
    const root = testRoot();
    const edits = files.map(([path, text]) => Edit.setText(path, text));
    const source = Source.memory(root, edits);

    return new RepositoryFixture(await open(source), root);
}

export const workspaces = [
    ["public", openPublicWorkspace],
    ["napi", openNapiWorkspace],
    ["wasm", openWasmWorkspaceInNode],
];

export const repositories = [
    ["public", openPublicRepository],
    ["napi", openNapiRepository],
    ["wasm", openWasmRepositoryInNode],
];

let wasmReady;

function testRoot() {
    return mkdtempSync(join(tmpdir(), "destack-bridge-typescript-tests-"));
}

async function openWasmWorkspaceInNode(source) {
    await initializeWasm();

    return openWasmWorkspace(source);
}

async function openWasmRepositoryInNode(source) {
    await initializeWasm();

    return openWasmRepository(source);
}

async function initializeWasm() {
    // initialize wasm once per test process
    if (!wasmReady) {
        const wasmPath = new URL("../../wasm/dist/destack_wasm_bg.wasm", import.meta.url);
        const bytes = await readFile(wasmPath);

        // use bytes because Node cannot fetch file urls
        wasmReady = wasm.default({ module_or_path: bytes });
    }

    await wasmReady;
}
