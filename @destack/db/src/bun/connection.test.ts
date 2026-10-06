import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { socketChannel } from "../channel/socket.ts";
import { defineTable, text } from "../index.ts";
import { connectBunSqlite } from "./connection.ts";

/** A logged setting without a scope column. */
const setting = defineTable(
    "setting",
    { name: text("name").primaryKey(), value: text("value").notNull() },
    { log: {} },
);

test("wake a file's readers on a commit another process announces on the file's channel", async () => {
    // open the file here, with its log under a space's scope
    const directory = await mkdtemp(join(tmpdir(), "destack-sqlite-channel-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const path = join(directory, "database.db");
    const reader = await connectBunSqlite(path, [setting], {
        openChannel: (name) => socketChannel(`${path}#${name}`),
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
        import { Database } from "bun:sqlite";
        import { socketChannel } from ${JSON.stringify(join(import.meta.dirname, "../channel/socket.ts"))};
        const database = new Database(${JSON.stringify(path)});
        database.run("INSERT INTO destack__db__setting (name, value) VALUES ('theme', 'dark')");
        database.close();
        const channel = socketChannel(${JSON.stringify(`${path}#log`)});
        const stop = channel.listen(() => {}, () => {
            channel.notify({ kind: "commit" });
            setTimeout(() => { stop(); }, 50);
        });
    `;
    const child = Bun.spawn([process.execPath, "-e", script], { stderr: "pipe" });
    expect([await child.exited, await new Response(child.stderr).text()]).toEqual([0, ""]);
    expect(await waiting).toBe(true);
});
