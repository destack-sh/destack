import { test, expect } from "@destack/test";
import { chmod, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { text } from "node:stream/consumers";
import { Sandbox } from "./sandbox.ts";
import { once } from "node:events";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

test("isolate filesystem access, environment and child processes", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-")));
    const permitted = join(directory, "permitted.txt");
    const forbidden = join(directory, "forbidden.txt");
    await writeFile(permitted, "permitted");
    await writeFile(forbidden, "private");

    try {
        // execute the actual runtime with only one readable application file
        await using sandbox = await Sandbox.start({
            executable: process.execPath,
            arguments: [
                "--no-env-file",
                "-e",
                `
                import { readFile, writeFile } from "node:fs/promises";
                const failures = [];
                for (const operation of [
                    () => readFile(${JSON.stringify(forbidden)}),
                    () => writeFile(${JSON.stringify(permitted)}, "changed"),
                ]) {
                    try { await operation(); failures.push("allowed"); }
                    catch (error) { failures.push(error.code); }
                }
                const child = Bun.spawn(["/bin/cat", ${JSON.stringify(forbidden)}], { stdout: "pipe", stderr: "ignore" });
                const childCode = await child.exited;
                console.log(JSON.stringify({
                    content: await readFile(${JSON.stringify(permitted)}, "utf8"),
                    failures,
                    childCode,
                    environment: process.env.DESTACK_SANDBOX_TEST,
                    inherited: process.env.USER ?? null,
                }));
            `,
            ],
            directory,
            environment: { DESTACK_SANDBOX_TEST: "explicit" },
            read: [permitted],
            write: [],
            network: [],
        });
        const output = text(sandbox.stdout);
        const error = text(sandbox.stderr);
        expect(await sandbox.exited, await error).toEqual({ code: 0, signal: null });
        expect(JSON.parse(await output)).toEqual({
            content: "permitted",
            failures: ["EPERM", "EPERM"],
            childCode: 1,
            environment: "explicit",
            inherited: null,
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("enforce independent network permissions for concurrent workloads", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-network-")));
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("allowed"),
    });
    const url = `http://127.0.0.1:${server.port}`;

    try {
        // run the same client under two independent proxy policies
        const results = await Promise.all(
            [true, false].map(async (allowed) => {
                await using sandbox = await Sandbox.start({
                    executable: process.execPath,
                    arguments: [
                        "--no-env-file",
                        "-e",
                        `
                    const url = ${JSON.stringify(url)};
                    const { connect } = await import("node:net");
                    const direct = await new Promise((resolve) => {
                        const socket = connect({ host: "127.0.0.1", port: ${server.port} });
                        socket.once("connect", () => { socket.destroy(); resolve("allowed"); });
                        socket.once("error", () => resolve("denied"));
                    });
                    const proxy = process.env.HTTP_PROXY?.replace("localhost", "127.0.0.1");
                    let result = { direct, status: null, text: null };
                    if (proxy) {
                        const response = await fetch(url, { proxy, signal: AbortSignal.timeout(1000) });
                        result = { direct, status: response.status, text: response.ok ? await response.text() : null };
                    }
                    console.log(JSON.stringify(result));
                `,
                    ],
                    directory,
                    environment: {},
                    read: [],
                    write: [],
                    network: allowed ? [`127.0.0.1:${server.port}`] : [],
                });
                const output = text(sandbox.stdout);
                const errors = text(sandbox.stderr);
                expect(await sandbox.exited, await errors).toEqual({ code: 0, signal: null });

                return JSON.parse(await output);
            }),
        );
        expect(results).toEqual([
            { direct: "denied", status: 200, text: "allowed" },
            { direct: "denied", status: 403, text: null },
        ]);
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true });
    }
});

test("stop a running workload and complete graceful shutdown", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-stop-")));
    try {
        await using sandbox = await Sandbox.start({
            executable: process.execPath,
            arguments: [
                "--no-env-file",
                "-e",
                `
                process.on("SIGTERM", () => { console.log("stopped"); process.exit(0); });
                setInterval(() => {}, 1000);
                console.log("ready");
            `,
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
        });
        const ready = once(sandbox.stdout, "data");
        const output = text(sandbox.stdout);
        const errors = text(sandbox.stderr);
        await ready;
        const stopping = sandbox.stop(1000);
        expect(sandbox.stop(0)).toBe(stopping);
        expect(await stopping, await errors).toEqual({ code: 0, signal: null });
        expect(await output).toBe("ready\nstopped\n");
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("preserve literal arguments and application failure after cleanup", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-exit-")));
    const argument = "quote' dollar$ semicolon; newline\n literal";
    try {
        await using sandbox = await Sandbox.start({
            executable: process.execPath,
            arguments: [
                "--no-env-file",
                "-e",
                "console.log(process.argv.at(-1)); process.exit(7)",
                argument,
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
        });
        const output = text(sandbox.stdout);
        const errors = text(sandbox.stderr);
        expect(await sandbox.exited, await errors).toEqual({ code: 7, signal: null });
        expect(await output).toBe(`${argument}\n`);
        expect(await sandbox.stop()).toEqual({ code: 7, signal: null });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("terminate a workload that ignores graceful shutdown", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-kill-")));
    try {
        await using sandbox = await Sandbox.start({
            executable: process.execPath,
            arguments: [
                "--no-env-file",
                "-e",
                "process.on('SIGTERM', () => {}); setInterval(() => {}, 1000); console.log('ready')",
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
        });
        const ready = once(sandbox.stdout, "data");
        const output = text(sandbox.stdout);
        const errors = text(sandbox.stderr);
        await ready;
        expect(await sandbox.stop(20), await errors).toEqual({ code: null, signal: "SIGKILL" });
        expect(await output).toBe("ready\n");
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("reject completion when temporary storage cleanup fails", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-cleanup-")));
    let temporary: string | undefined;
    try {
        // make a private subdirectory unreadable before exiting successfully
        const sandbox = await Sandbox.start({
            executable: process.execPath,
            arguments: [
                "--no-env-file",
                "-e",
                `
                import {mkdir, writeFile, chmod} from 'node:fs/promises';
                const directory = process.env.TMPDIR + '/locked';
                await mkdir(directory);
                await writeFile(directory + '/file', 'private');
                await chmod(directory, 0);
                console.log(process.env.TMPDIR);
            `,
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
        });
        const failed = expect(sandbox.exited).rejects.toMatchObject({
            name: "SandboxError",
            code: "STOP_FAILED",
            message: "sandbox launcher terminated without successful cleanup",
        });
        const errors = text(sandbox.stderr);
        temporary = (await text(sandbox.stdout)).trim();
        await failed;
        await errors;
    } finally {
        if (temporary) {
            await chmod(join(temporary, "locked"), 0o700);
            await rm(temporary, { recursive: true });
        }
        await rm(directory, { recursive: true });
    }
});

test("stop the workload when its supervising client disconnects", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-sandbox-parent-")));
    const launcher = spawn(
        process.execPath,
        ["run", "--no-env-file", fileURLToPath(new URL("../main.ts", import.meta.url))],
        {
            cwd: directory,
            env: { PATH: "/usr/bin:/bin:/usr/sbin:/sbin", HOME: directory },
            stdio: ["ignore", "pipe", "pipe", "ipc"],
        },
    );
    const closed = once(launcher, "close");
    const errors = text(launcher.stderr!);
    try {
        // disconnect only after the restricted application has started
        const ready = once(launcher.stdout!, "data");
        launcher.send({
            type: "start",
            options: {
                executable: process.execPath,
                arguments: [
                    "--no-env-file",
                    "-e",
                    "console.log(process.pid); setInterval(() => {}, 1000)",
                ],
                directory,
                environment: {},
                read: [],
                write: [],
                network: [],
            },
        });
        const [chunk] = await ready;
        const pid = Number(chunk.toString().trim());
        launcher.disconnect();

        // wait for the launcher to reap its workload and release the proxies
        expect(await closed, await errors).toEqual([0, null]);
        expect(() => process.kill(pid, 0)).toThrowError(expect.objectContaining({ code: "ESRCH" }));
    } finally {
        if (launcher.connected) {
            launcher.disconnect();
        }
        await closed;
        await rm(directory, { recursive: true });
    }
});
