import { expect, test } from "bun:test";

import type { CheckInput } from "../dist/_generated/workspace/command/check.js";
import type { InfoInput } from "../dist/_generated/workspace/command/info.js";
import type { Json, Workspace } from "../dist/index.js";
import { openWorkspace } from "../dist/index.js";
import { memoryFiles, memoryRoot } from "../dist/workspace/memory.js";

/** One client workspace test project. */
type Project = {
    /** Typed `destack.json` content. */
    readonly config: Json;
    /** Repository relative text files. */
    readonly files: Readonly<Record<string, string>>;
};

test("server stream terminates with its command response", async () => {
    const workspace = await openProject({
        config: { name: "@test/app" },
        files: {},
    });

    try {
        const call = workspace.info(infoInput());
        const progress = [];
        for await (const event of call) {
            progress.push(event);
        }
        const response = await call.response();

        expect(response.value.success).toBe(true);
        expect(response.value.diagnostics).toEqual([]);
        expect(response.value.data.workspace.root).toBe("/workspace");
        expect(progress).toEqual([]);
    } finally {
        workspace.connection.close();
    }
});

test("compiler work advances through cooperative N-API polls", async () => {
    const workspace = await openProject({
        config: {
            name: "@test/app",
            targets: { default: { entry: ["main.ds"] } },
            defaultTarget: "default",
        },
        files: { "main.ds": "export const value: int32 = 1;\n" },
    });

    try {
        const call = workspace.check(checkInput());
        const progress = [];
        for await (const event of call) {
            progress.push(event);
        }
        const response = await call.response();

        expect(response.value.success).toBe(true);
        expect(response.value.diagnostics).toEqual([]);
        expect(progress.length).toBeGreaterThan(0);
    } finally {
        workspace.connection.close();
    }
});

test("compiler rechecks an edited workspace revision incrementally", async () => {
    const workspace = await openProject({
        config: {
            name: "@test/app",
            targets: { default: { entry: ["main.ds", "util.ds"] } },
            defaultTarget: "default",
        },
        files: {
            "main.ds": "export const value: int32 = 1;\n",
            "util.ds": "export const helper: int32 = 1;\n",
        },
    });

    try {
        const before = await workspace.revision();
        const firstCall = workspace.check({ ...checkInput(), trace: "detailed" });
        for await (const _event of firstCall) {
            // drain progress until the terminal response
        }
        const first = await firstCall.response();
        const commit = await workspace.applySourceUpdate({
            base: before.value,
            edits: [
                {
                    kind: "setText",
                    path: "main.ds",
                    text: "export const value: int32 = 2;\n",
                },
            ],
        });

        const secondCall = workspace.check({
            ...checkInput(),
            trace: "detailed",
        });
        for await (const _event of secondCall) {
            // drain progress until the terminal response
        }
        const second = await secondCall.response();
        const after = await workspace.revision();
        const firstTrace = first.value.trace;
        const secondTrace = second.value.trace;

        expect(first.value.success).toBe(true);
        expect(second.value.success).toBe(true);
        expect(commit.value.before).toEqual(before.value);
        expect(commit.value.after).toEqual(after.value);
        expect(secondTrace?.stats.built).toBeGreaterThan(0n);
        expect(secondTrace?.stats.built).toBeLessThan(firstTrace?.stats.built ?? 0n);
    } finally {
        workspace.connection.close();
    }
});

test("workspace watch emits its exact revision and later commits", async () => {
    const workspace = await openProject({
        config: { name: "@test/app" },
        files: { "main.ds": "export const value = 1;\n" },
    });
    const watch = workspace.watch();

    try {
        // establish the root subscription before changing source
        const before = await workspace.revision();
        const ready = await watch.receive();
        expect(ready).toEqual({ kind: "ready", revision: before.value });

        // observe the same commit returned by the mutating operation
        const commit = await workspace.applySourceUpdate({
            base: before.value,
            edits: [
                {
                    kind: "setText",
                    path: "main.ds",
                    text: "export const value = 2;\n",
                },
            ],
        });
        const event = await watch.receive();

        expect(event).toEqual({ kind: "commit", commit: commit.value });
    } finally {
        await watch.cancel();
        workspace.connection.close();
    }
});

test("memory workspace inputs normalize config and files", () => {
    const workspace = {
        memory: {
            root: "/project",
            config: { name: "@test/app" },
            files: {
                "src/index.ds": "export const value = 1;\n",
                "asset.bin": [1, 2, 3],
            },
        },
    };

    const files = memoryFiles(workspace);

    expect(memoryRoot(workspace)).toBe("/project");
    expect(files).toEqual([
        { path: "destack.json", text: "{\"name\":\"@test/app\"}\n" },
        { path: "src/index.ds", text: "export const value = 1;\n" },
        { path: "asset.bin", bytes: new Uint8Array([1, 2, 3]) },
    ]);
});

test("memory workspace inputs reject ambiguous files", () => {
    const workspace = {
        memory: {
            files: [
                {
                    path: "src/index.ds",
                    text: "export const value = 1;\n",
                    bytes: new Uint8Array([1]),
                },
            ],
        },
    };

    expect(() => memoryFiles(workspace)).toThrow("text and bytes");
});

/** Open one in-memory project through the native in-process RPC session. */
async function openProject(project: Project): Promise<Workspace> {
    return openWorkspace({
        memory: {
            config: project.config,
            files: project.files,
        },
    });
}

/** Return one exact workspace information input. */
function infoInput(): InfoInput {
    return {
        revision: { kind: "current" },
        inputs: [],
        configInputs: false,
        env: [],
        overrides: [],
        watch: false,
        dryRun: true,
        all: false,
    };
}

/** Return one exact workspace check input. */
function checkInput(): CheckInput {
    return {
        revision: { kind: "current" },
        inputs: [],
        configInputs: true,
        env: [],
        overrides: [],
        watch: false,
        dryRun: true,
        lint: false,
        fix: false,
        unsafeFixes: false,
        diff: false,
    };
}
