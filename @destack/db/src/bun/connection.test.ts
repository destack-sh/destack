import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { socketChannel } from "../channel/socket.ts";
import { setting } from "./test/setting.ts";
import { connectBunSqlite } from "./connection.ts";
import { testHost } from "@destack/test/host";

/** The preload that gives a child process's modules their package. */
const PRELOAD = "@destack/package/bun/preload";

/** The host the processes share a file on. */
const HOST = testHost();

test("wake a file's readers on a commit another process announces on the file's channel", async () => {
    // open the file here, with its log under a space's scope
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-channel-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const reader = await connectBunSqlite(path, [setting], {
        environment: HOST,
        openChannel: (name) => socketChannel(`${path}#${name}`, HOST),
    });
    onTestFinished(() => reader.close());
    await reader.log.create("space-a");
    await reader.migrate([setting]);

    // wait here while another process writes the file and announces its commit
    const waiting = reader.log.until(
        async () => (await reader.log.position()).sequence > 0,
        AbortSignal.timeout(5000),
    );
    const script = `
        import { socketChannel } from ${JSON.stringify(join(import.meta.dirname, "../channel/socket.ts"))};
        import { connectBunSqlite } from ${JSON.stringify(join(import.meta.dirname, "connection.ts"))};
        import { setting } from ${JSON.stringify(join(import.meta.dirname, "test/setting.ts"))};
        import { BunEnvironment } from "@destack/environment/bun";
        const host = BunEnvironment.system;
        const database = await connectBunSqlite(${JSON.stringify(path)}, [setting], { environment: host });
        await database.transaction((transaction) => transaction.insert(setting).values({ name: "theme", value: "dark" }).execute());
        await database.close();
        const channel = socketChannel(${JSON.stringify(`${path}#log`)}, host);
        const stop = channel.listen(() => {}, () => {
            channel.notify({ kind: "commit", tables: ["destack__db__setting"] });
            void host.clock.sleep(50).then(stop);
        });
    `;
    const child = HOST.process.spawn(
        [HOST.process.executable, "--preload", PRELOAD, "-e", script],
        {
            stderr: "pipe",
        },
    );
    expect([await child.exited, await new Response(child.stderr).text()]).toEqual([0, ""]);
    expect(await waiting).toBe(true);
});
