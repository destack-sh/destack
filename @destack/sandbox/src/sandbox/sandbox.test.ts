import { join } from "node:path";
import type { OperatingSystem } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { Simulation } from "@destack/environment/simulation";
import { test, expect, refusal } from "@destack/test";
import { launcherCommand } from "../bun/command.ts";
import { Sandbox } from "./sandbox.ts";

/** The errors a denied read and a denied write report, by platform. */
const DENIED_FAILURES: Partial<Record<OperatingSystem, readonly string[]>> = {
    darwin: ["EPERM", "EPERM"],
    linux: ["ENOENT", "EROFS"],
};

/** The start of the worlds' clocks. */
const START = Date.UTC(2026, 9, 24);

/** The search path of a launcher a test starts itself. */
const LAUNCHER_PATH = "/usr/bin:/bin:/usr/sbin:/sbin";

/** Make a world on the host's platform, and a real temporary directory in it. */
async function world(prefix: string) {
    const simulation = new Simulation({
        seed: 1,
        start: START,
        host: testHost().host,
    });
    const environment = simulation.environment("host");
    const { files } = environment;
    const directory = await files.realPath(await files.makeTemporaryDirectory(prefix));

    return { simulation, environment, directory };
}

/** Read a stream's text up to its end. */
function text(stream: ReadableStream<Uint8Array>): Promise<string> {
    return new Response(stream).text();
}

/** Read a stream's first chunk, and its whole text once it ends. */
function split(stream: ReadableStream<Uint8Array>): {
    first: Promise<string>;
    whole: Promise<string>;
} {
    const [head, whole] = stream.tee();
    const first = head
        .getReader()
        .read()
        .then(({ value }) => new TextDecoder().decode(value));

    return { first, whole: text(whole) };
}

test("isolate filesystem access, environment and child processes", async () => {
    // write a readable and a private file in a world
    const { simulation, environment, directory } = await world("destack-sandbox-");
    await using _simulation = simulation;
    const { files, process, host } = environment;
    const permitted = join(directory, "permitted.txt");
    const forbidden = join(directory, "forbidden.txt");
    await files.writeTextFile(permitted, "permitted");
    await files.writeTextFile(forbidden, "private");

    // execute the actual runtime with only one readable application file
    await using sandbox = await Sandbox.start(
        {
            executable: process.executable,
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
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = text(sandbox.stdout);
    const error = text(sandbox.stderr);
    expect(await sandbox.exited, await error).toEqual({ code: 0, signal: null });
    expect(JSON.parse(await output)).toEqual({
        content: "permitted",
        failures: DENIED_FAILURES[host.os],
        childCode: 1,
        environment: "explicit",
        inherited: null,
    });
});

test("start the workload without the launcher's parent connection on file descriptor 3", async () => {
    // report from inside the sandbox whether descriptor 3 takes a write
    const { simulation, environment, directory } = await world("destack-sandbox-descriptor-");
    await using _simulation = simulation;
    await using sandbox = await Sandbox.start(
        {
            executable: "/bin/sh",
            arguments: ["-c", "if { : >&3; } 2>/dev/null; then echo open; else echo closed; fi"],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = text(sandbox.stdout);
    const error = text(sandbox.stderr);
    expect(await sandbox.exited, await error).toEqual({ code: 0, signal: null });
    expect(await output).toBe("closed\n");
});

test("enforce independent network permissions for concurrent workloads", async () => {
    // serve a page on a real loopback port, which a world's network cannot offer a sandbox
    const { simulation, environment, directory } = await world("destack-sandbox-network-");
    await using _simulation = simulation;
    const server = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("allowed"),
    });
    const { port } = server.url;
    const url = `http://127.0.0.1:${port}`;

    try {
        // run the same client under two independent proxy policies
        const results = await Promise.all(
            [true, false].map(async (allowed) => {
                await using sandbox = await Sandbox.start(
                    {
                        executable: environment.process.executable,
                        arguments: [
                            "--no-env-file",
                            "-e",
                            `
                    const url = ${JSON.stringify(url)};
                    const { connect } = await import("node:net");
                    const direct = await new Promise((resolve) => {
                        const socket = connect({ host: "127.0.0.1", port: ${port} });
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
                        network: allowed ? [`127.0.0.1:${port}`] : [],
                        sockets: [],
                        listening: "denied",
                    },
                    environment,
                );
                const output = text(sandbox.stdout);
                const errors = text(sandbox.stderr);
                expect(await sandbox.exited, await errors).toEqual({ code: 0, signal: null });

                const parsed: unknown = JSON.parse(await output);

                return parsed;
            }),
        );
        expect(results).toEqual([
            { direct: "denied", status: 200, text: "allowed" },
            { direct: "denied", status: 403, text: null },
        ]);
    } finally {
        await server.stop(true);
    }
});

test("stop a running workload and complete graceful shutdown", async () => {
    // start a workload that prints once it runs, and again once it stops
    const { simulation, environment, directory } = await world("destack-sandbox-stop-");
    await using _simulation = simulation;
    await using sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
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
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = split(sandbox.stdout);
    const errors = text(sandbox.stderr);
    await output.first;

    // refuse an invalid grace period, then share one stop
    expect(await refusal(() => sandbox.stop(-1))).toEqual([
        "BAD_REQUEST",
        "GRACE_PERIOD_INVALID",
        "grace period must be between 0 and 300000 milliseconds",
    ]);
    const stopping = sandbox.stop(1000);
    expect(sandbox.stop(0)).toBe(stopping);
    expect(await stopping, await errors).toEqual({ code: 0, signal: null });
    expect(await output.whole).toBe("ready\nstopped\n");
});

test("preserve literal arguments and application failure after cleanup", async () => {
    // run a workload that prints its last argument and fails
    const { simulation, environment, directory } = await world("destack-sandbox-exit-");
    await using _simulation = simulation;
    const argument = "quote' dollar$ semicolon; newline\n literal";
    await using sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
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
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = text(sandbox.stdout);
    const errors = text(sandbox.stderr);
    expect(await sandbox.exited, await errors).toEqual({ code: 7, signal: null });
    expect(await output).toBe(`${argument}\n`);
    expect(await sandbox.stop()).toEqual({ code: 7, signal: null });
});

test("terminate a workload that ignores graceful shutdown", async () => {
    // start a workload that ignores SIGTERM
    const { simulation, environment, directory } = await world("destack-sandbox-kill-");
    await using _simulation = simulation;
    await using sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
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
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = split(sandbox.stdout);
    const errors = text(sandbox.stderr);
    await output.first;
    expect(await sandbox.stop(20), await errors).toEqual({ code: null, signal: "SIGKILL" });
    expect(await output.whole).toBe("ready\n");
});

test("reject completion when temporary storage cleanup fails", async () => {
    // make a private subdirectory unreadable before exiting successfully
    const { simulation, environment, directory } = await world("destack-sandbox-cleanup-");
    await using _simulation = simulation;
    const { files } = environment;
    const sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
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
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const failed = refusal(sandbox.exited);
    const errors = text(sandbox.stderr);
    const temporary = (await text(sandbox.stdout)).trim();
    try {
        expect(await failed).toEqual([
            "INTERNAL_SERVER_ERROR",
            "STOP_FAILED",
            "cannot remove the sandbox's temporary directory",
        ]);
        await errors;
    } finally {
        await files.chmod(join(temporary, "locked"), 0o700);
        await files.remove(temporary, { recursive: true });
    }
});

test("stop the workload when its supervising client disconnects", async () => {
    // start a launcher by hand, connected, with its own temporary directory
    const { simulation, environment, directory } = await world("destack-sandbox-parent-");
    await using _simulation = simulation;
    const { files, process } = environment;
    const temporary = await files.realPath(await files.makeTemporaryDirectory("launcher-"));
    const command = launcherCommand(process);
    const launcher = process.fork([command.executable, ...command.arguments], {
        workingDirectory: directory,
        variables: {
            PATH: LAUNCHER_PATH,
            HOME: temporary,
            TMPDIR: temporary,
            CLAUDE_CODE_TMPDIR: temporary,
        },
        stdout: "pipe",
        stderr: "pipe",
    });
    const output = split(launcher.stdout);
    const errors = text(launcher.stderr);

    // disconnect only after the restricted application has started
    launcher.channel.send({
        type: "start",
        options: {
            executable: process.executable,
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
            sockets: [],
            listening: "denied",
        },
    });
    const id = Number((await output.first).trim());
    launcher.channel.disconnect();

    // wait for the launcher to reap its workload and release the proxies
    expect({ code: await launcher.exited, signal: launcher.signal }, await errors).toEqual({
        code: 0,
        signal: undefined,
    });

    // find no process under the workload's id
    const listing = process.spawn(["/bin/ps", "-p", String(id)], { stdout: "ignore" });
    expect(await listing.exited).toBe(1);
});

test("pass input to a workload and let it serve its host on a loopback port when allowed", async () => {
    // start a workload that reads its greeting and serves it on a loopback port it reports
    const { simulation, environment, directory } = await world("destack-sandbox-");
    await using _simulation = simulation;
    await using sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
            arguments: [
                "--no-env-file",
                "-e",
                `
                const greeting = (await new Response(Bun.stdin.stream()).text()).trim();
                const server = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: () => new Response(greeting) });
                console.log(server.port);
            `,
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
            sockets: [],
            listening: "loopback",
        },
        environment,
    );
    const writer = sandbox.stdin.getWriter();
    await writer.write(new TextEncoder().encode("hello\n"));
    await writer.close();

    // read the port from the first chunk and fetch the greeting from it
    const { value } = await sandbox.stdout.getReader().read();
    const port = new TextDecoder().decode(value).trim();
    const answer = await testHost().network.fetch(new Request(`http://127.0.0.1:${port}/`));
    expect(await answer.text()).toBe("hello");
});

test("let a workload serve and reach a Unix socket in its temporary folder", async () => {
    // serve a socket below the temporary folder, and connect to it
    const { simulation, environment, directory } = await world("destack-sandbox-socket-");
    await using _simulation = simulation;
    await using sandbox = await Sandbox.start(
        {
            executable: environment.process.executable,
            arguments: [
                "--no-env-file",
                "-e",
                `
                const { createServer, connect } = await import("node:net");
                const path = process.env.TMPDIR + "/channel.sock";
                const server = createServer((peer) => peer.end("joined"));
                await new Promise((resolve) => server.listen(path, resolve));
                const answer = await new Promise((resolve, reject) => {
                    const socket = connect(path);
                    let text = "";
                    socket.on("data", (chunk) => (text += chunk));
                    socket.once("end", () => resolve(text));
                    socket.once("error", reject);
                });
                server.close();
                console.log(answer);
            `,
            ],
            directory,
            environment: {},
            read: [],
            write: [],
            network: [],
            sockets: [],
            listening: "denied",
        },
        environment,
    );
    const output = text(sandbox.stdout);
    const errors = text(sandbox.stderr);
    expect(await sandbox.exited, await errors).toEqual({ code: 0, signal: null });
    expect(await output).toBe("joined\n");
});
