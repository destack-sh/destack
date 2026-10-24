import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { expect, refusal, test } from "@destack/test";
import { Crypto, type Host } from "../src/index.ts";
import { Simulation } from "../src/simulation/index.ts";

/** The simulations' start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("write a node's files at their host paths without moving its clock, and remove the world's root on dispose", async () => {
    // write a file in a temporary directory while a later timer waits
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    void simulation.clock.sleep(1);
    let directory = "";
    const writing = files.makeTemporaryDirectory("notes-").then(async (made) => {
        await files.writeTextFile(join(made, "today.md"), "plans");
        directory = made;
    });
    await simulation.run({ until: () => directory !== "" });
    await writing;
    const writtenAt = simulation.clock.now();

    // read it through the files and on the host, then dispose the world
    const text = await files.readTextFile(join(directory, "today.md"));
    const host = await readFile(join(directory, "today.md"), "utf8");
    const entries = await files.readDirectory(directory);
    await simulation[Symbol.asyncDispose]();

    // hold time at the start until the write settled, and leave no files behind
    expect({
        writtenAt,
        directory: relative(simulation.root, directory),
        text,
        host,
        entries,
        isRemoved: !existsSync(simulation.root),
    }).toEqual({
        writtenAt: START,
        directory: "notes-dhgozk",
        text: "plans",
        host: "plans",
        entries: [{ name: "today.md", kind: "file" }],
        isRemoved: true,
    });
});

test("spawn a child with the node's variables in a directory, its exit an untracked wait", async () => {
    // spawn a shell printing its variable and working directory in a temporary directory
    await using simulation = new Simulation({
        seed: 1,
        start: START,
        process: {
            arguments: [],
            variables: { GREETING: "hello" },
            workingDirectory: "/",
            id: 1,
            userId: 501,
            executable: "/usr/local/bin/bun",
            runtime: { revision: "0".repeat(40), isStandalone: false },
        },
    });
    const node = simulation.environment("host");
    const workingDirectory = await node.files.makeTemporaryDirectory("work-");
    const child = node.process.spawn(["/bin/sh", "-c", 'printf "$GREETING from $PWD"'], {
        workingDirectory,
    });
    const during = simulation.clock.inFlight;

    // read its output and exit code, the clock counting no work
    const [output, code] = await Promise.all([new Response(child.stdout).text(), child.exited]);

    expect({ during, output, code }).toEqual({
        during: 0,
        output: `hello from ${workingDirectory}`,
        code: 0,
    });
});

test("accept requests and TCP connections a node serves from another node", async () => {
    // serve a request and accept a TCP connection on the host
    await using simulation = new Simulation({ seed: 1, start: START });
    const host = simulation.environment("host");
    const listener = host.server.http({
        hostname: "notes.test",
        port: 443,
        fetch: (request) => new Response(`accepted ${new URL(request.url).pathname}`),
    });
    const accepted: string[] = [];
    const tcp = await host.server.tcp({
        hostname: "mail.test",
        port: 25,
        accept: () => accepted.push("mail"),
    });

    // reach both from the client
    const client = simulation.environment("client").network;
    const response = await client.fetch(new Request("https://notes.test/today"));
    const socket = client.connect({ hostname: "mail.test", port: 25 });
    await socket.opened;

    expect({
        url: listener.url.href,
        body: await response.text(),
        address: tcp.address,
        accepted,
    }).toEqual({
        url: "http://notes.test:443/",
        body: "accepted /today",
        address: { hostname: "mail.test", port: 25 },
        accepted: ["mail"],
    });
});

test("read the process and machine a simulation gives", async () => {
    // give a node its process and machine
    const host: Host = {
        os: "darwin",
        architecture: "arm64",
        version: "24.1.0",
        hostname: "laptop",
        home: "/Users/ada",
    };
    await using simulation = new Simulation({
        seed: 1,
        start: START,
        process: {
            arguments: ["sync", "--watch"],
            variables: { DESTACK_HOME: "/Users/ada/.destack" },
            workingDirectory: "/Users/ada",
            id: 42,
            userId: 501,
            executable: "/usr/local/bin/destack",
            runtime: { revision: "a".repeat(40), isStandalone: true },
        },
        host,
    });
    const node = simulation.environment("laptop");

    expect({
        host: node.host,
        arguments: node.process.arguments,
        variables: node.process.variables,
        workingDirectory: node.process.workingDirectory,
        id: node.process.id,
        userId: node.process.userId,
        executable: node.process.executable,
        missing: [
            node.process.which("destack-missing-command"),
            await node.process.hasLibrary("libdestack-missing.so"),
        ],
        runtime: node.process.runtime,
    }).toEqual({
        host,
        arguments: ["sync", "--watch"],
        variables: { DESTACK_HOME: "/Users/ada/.destack" },
        workingDirectory: "/Users/ada",
        id: 42,
        userId: 501,
        executable: "/usr/local/bin/destack",
        missing: [undefined, false],
        runtime: { revision: "a".repeat(40), isStandalone: true },
    });
});

test("count a node's WebCrypto operations on the clock until they settle, leaving the realm's own alone", async () => {
    // digest, reading the work in flight at once and once it settled
    await using simulation = new Simulation({ seed: 1, start: START });
    const subtle = simulation.environment("host").crypto;
    const digest = subtle.digest("SHA-256", new Uint8Array(4));
    const during = simulation.clock.inFlight;
    const bytes = (await digest).byteLength;

    expect({
        during,
        bytes,
        after: simulation.clock.inFlight,
        settled: simulation.clock.settled,
        isPatched: Object.hasOwn(crypto.subtle, "digest"),
    }).toEqual({ during: 1, bytes: 32, after: 0, settled: 1, isPatched: false });
});

test("compress and decompress gzip in a world at once, without moving its clock or leaving work in flight", async () => {
    // compress a text and decompress it again through a node's compression
    await using simulation = new Simulation({ seed: 1, start: START });
    const { compression } = simulation.environment("host");
    const text = "plans for today, plans for tomorrow";
    const compressed = new Blob([text]).stream().pipeThrough(compression.compress("gzip"));
    const restored = await new Response(
        compressed.pipeThrough(compression.decompress("gzip")),
    ).text();

    expect({ restored, at: simulation.clock.now(), inFlight: simulation.clock.inFlight }).toEqual({
        restored: text,
        at: START,
        inFlight: 0,
    });
});

test("record a world's spans and log records, and format times in its fixed locale and time zone", async () => {
    // emit a span and a log record from a node, and format the world's start
    await using simulation = new Simulation({ seed: 1, start: START, timeZone: "Europe/Vienna" });
    const node = simulation.environment("host");
    node.telemetry.tracers.getTracer("notes").startSpan("open").end();
    node.telemetry.loggers.getLogger("notes").emit({ eventName: "note.open", body: "today" });
    const formatted = new Date(START).toLocaleString(node.locale, { timeZone: node.timeZone });

    expect({
        spans: simulation.telemetry.spans.map((span) => [
            span.instrumentationScope.name,
            span.name,
        ]),
        logs: simulation.telemetry.logs.map((record) => [record.eventName, record.body]),
        formatted,
    }).toEqual({
        spans: [["notes", "open"]],
        logs: [["note.open", "today"]],
        formatted: "10/24/2026, 2:00:00 AM",
    });
});

test("spawn a detached child in its own process group, its output ignored", async () => {
    // start a sleeping child detached, and read its process group
    await using simulation = new Simulation({ seed: 1, start: START });
    const { process } = simulation.environment("host");
    const child = process.spawn(["/bin/sleep", "5"], { isDetached: true });
    const reading = process.spawn(["/bin/ps", "-o", "pgid=", "-p", String(child.id)]);
    const group = Number((await new Response(reading.stdout).text()).trim());
    const output = await new Response(child.stdout).text();
    child.kill();

    expect({ group: group === child.id, output }).toEqual({ group: true, output: "" });
});

test("kill a detached child's process group by its negated id, read the signal that ended it, and refuse the ended group", async () => {
    // start a sleeping child in its own group, then kill the group
    await using simulation = new Simulation({ seed: 1, start: START });
    const { process } = simulation.environment("host");
    const child = process.spawn(["/bin/sleep", "5"], { isDetached: true });
    const running = child.signal;
    process.kill(-child.id, "SIGKILL");
    const code = await child.exited;

    // refuse a signal to the ended group, and to a process the node did not start
    const ended = await refusal(() => process.kill(-child.id, "SIGKILL"));
    const foreign = await refusal(() => process.kill(2, "SIGTERM"));

    expect({ running, code, signal: child.signal, ended, foreign }).toEqual({
        running: undefined,
        code: 137,
        signal: "SIGKILL",
        ended: [
            "NOT_FOUND",
            "NOT_FOUND",
            `process ${-child.id} is not this node or a running child`,
        ],
        foreign: ["NOT_FOUND", "NOT_FOUND", "process 2 is not this node or a running child"],
    });
});

test("append a detached child's output and errors to one file, created private", async () => {
    // run a detached shell writing a line to each of its outputs, both into one log file
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files, process } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("log-");
    const log = join(directory, "child.log");
    const child = process.spawn(["/bin/sh", "-c", "echo out; echo error >&2"], {
        isDetached: true,
        stdout: { file: log },
        stderr: { file: log },
    });
    const code = await child.exited;

    expect({
        code,
        text: await files.readTextFile(log),
        mode: (await files.stat(log)).mode & 0o777,
    }).toEqual({ code: 0, text: "out\nerror\n", mode: 0o600 });
});

test("hash a stream of chunks as its whole bytes, natively and by gathering, counted on a world's clock", async () => {
    // hash three chunks and their joined bytes with a node's crypto and the system's
    await using simulation = new Simulation({ seed: 1, start: START });
    const { crypto } = simulation.environment("host");
    const whole = new TextEncoder().encode("hash these chunks");

    expect({
        native: hex(await crypto.digest("SHA-256", stream())),
        gathered: hex(await Crypto.system.digest("SHA-256", stream())),
        whole: hex(await crypto.digest("SHA-256", whole)),
        refused: await crypto.digest("MD5", stream()).catch((error: unknown) => String(error)),
    }).toEqual({
        native: hex(await Crypto.system.digest("SHA-256", whole)),
        gathered: hex(await Crypto.system.digest("SHA-256", whole)),
        whole: hex(await Crypto.system.digest("SHA-256", whole)),
        refused: "TypeError: cannot hash a stream with MD5",
    });
});

/** Stream "hash these chunks" in three chunks. */
async function* stream(): AsyncGenerator<Uint8Array> {
    for (const text of ["hash ", "these ", "chunks"]) {
        yield new TextEncoder().encode(text);
    }
}

/** Write a digest as hexadecimal. */
function hex(digest: ArrayBuffer): string {
    return new Uint8Array(digest).toHex();
}
