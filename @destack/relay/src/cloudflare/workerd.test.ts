import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { modulePlugin } from "@destack/package/bun";
import { expect, onTestFinished, test } from "@destack/test";
import { TUNNEL_PATH } from "../server/index.ts";
import { freePort, until } from "../test/index.ts";
import { TunnelClient } from "../tunnel/index.ts";
import { NAME, RENAME_PATH } from "./test/fixture.ts";

/** The machine's new name the scenarios rename it to. */
const RENAMED = "renamed.acme.destack.computer";

/** Run the test relay's Worker with its machines' objects under workerd, answering its tunnel URL. */
async function start(): Promise<{ readonly worker: Miniflare; readonly url: string }> {
    // bundle the worker as Wrangler resolves workerd modules
    const compiled = await Bun.build({
        entrypoints: [fileURLToPath(new URL("./test/worker.ts", import.meta.url))],
        format: "esm",
        target: "browser",
        conditions: ["workerd", "worker", "browser"],
        external: ["cloudflare:*"],
        plugins: [modulePlugin],
    });
    const [output] = compiled.outputs;
    if (output === undefined) {
        throw new Error("the worker bundled into no file");
    }

    // listen on a port the machine dials
    const port = await freePort();
    const url = `http://127.0.0.1:${port}${TUNNEL_PATH}`;
    const worker = new Miniflare({
        modules: true,
        script: await output.text(),
        compatibilityDate: "2026-07-30",
        compatibilityFlags: ["nodejs_compat"],
        host: "127.0.0.1",
        port,
        durableObjects: { TUNNEL: { className: "TestTunnel", useSQLite: true } },
        bindings: { TUNNEL_URL: url },
    });
    onTestFinished(() => worker.dispose());
    await worker.ready;

    return { worker, url };
}

/** Dial the test relay as its machine with its tokens and liveness interval, answering each request with its path and keeping the names it learns. */
async function dial(url: string, token: () => Promise<string>, heartbeat: number) {
    const names: string[] = [];
    const reports: unknown[] = [];
    const client = TunnelClient.open({
        url,
        token,
        fetch: async (request) => new Response(`hello ${new URL(request.url).pathname}`),
        name: (name) => names.push(name),
        heartbeat,
        retry: { initialInterval: 60_000, maximumInterval: 60_000 },
        report: (error) => reports.push(error),
    });
    onTestFinished(() => client.close());
    await client.opened();

    return { names, reports };
}

/** Request a path of a name through the relay's listener, answering the body or the status. */
async function ask(worker: Miniflare, path: string, name = NAME): Promise<string> {
    const listener = await worker.ready;
    const response = await fetch(new URL(path, listener), { headers: { host: name } });
    const body = await response.text();

    return response.ok ? body : `${response.status}`;
}

test("forward a request through a machine's object over RPC, renew its token there, and tell it a new name, refusing the old one", async () => {
    const { worker, url } = await start();
    const { names, reports } = await dial(url, async () => "60000", 60_000);

    // reach the machine and learn its name through a renewal
    const answered = await ask(worker, "/status");
    await until(() => names.includes(NAME));

    // tell it a new name through its object that refuses the old name
    const renamed = await worker.dispatchFetch(`http://relay.test${RENAME_PATH}?name=${RENAMED}`);
    await until(() => names.includes(RENAMED));
    expect([
        answered,
        renamed.status,
        await ask(worker, "/status"),
        await ask(worker, "/status", RENAMED),
        reports,
    ]).toEqual(["hello /status", 204, "421", "hello /status", []]);
});

test("answer a machine's liveness probes on its object's behalf, keeping one connection across them", async () => {
    const { worker, url } = await start();
    const { names, reports } = await dial(url, async () => "60000", 50);

    // outlive many probes that drop and dial again once unanswered
    await new Promise((resolve) => {
        setTimeout(resolve, 500);
    });
    expect([await ask(worker, "/status"), names, reports]).toEqual(["hello /status", [NAME], []]);
});

test("close a machine's connection on its object's alarm once its renewal fails and its token lapses, then refuse its requests", async () => {
    const { worker, url } = await start();
    let tokens = 0;
    const { names, reports } = await dial(
        url,
        async () => {
            tokens += 1;
            if (tokens > 1) {
                throw new TypeError("the issuer is unreachable");
            }

            return "300";
        },
        60_000,
    );

    // reach the machine until its token lapses without a renewal
    const before = await ask(worker, "/status");
    await until(async () => (await ask(worker, "/status")) === "503");
    expect([before, names, reports.at(0)]).toEqual([
        "hello /status",
        [],
        new TypeError("the issuer is unreachable"),
    ]);
});
