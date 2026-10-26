import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { channelHub } from "../channel/channel.ts";
import { socketChannel } from "../channel/socket.ts";
import { changeTables, first, note, revision } from "../log/test/fixture.ts";
import { Transaction } from "../log/commit.ts";
import { eq } from "../sql/index.ts";
import { present } from "@destack/schema";
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
        const stop = channel.listen({ receive: () => {}, resume: () => {
            channel.notify({ kind: "commit", tables: ["destack__db__setting"] });
            void host.clock.sleep(50).then(stop);
        } });
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

test("read anew what an unannounced commit wrote, once a position read finds it in the log", async () => {
    // open a file twice: a reader on a channel, and a writer that announces nothing
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-unheard-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const hub = channelHub<unknown>();
    const reader = await connectBunSqlite(path, changeTables, {
        environment: HOST,
        openChannel: () => hub(),
    });
    onTestFinished(() => reader.close());
    await reader.log.create("inbox");
    await reader.migrate(changeTables);
    const writer = await connectBunSqlite(path, changeTables, { environment: HOST });
    onTestFinished(() => writer.close());
    const revise = (number: number) =>
        reader.transaction((transaction) =>
            transaction
                .insert(revision)
                .values({ scope: "inbox", noteId: "a", number, title: "First" })
                .execute(),
        );

    // hear commits, catch the position up once, then read the notes from the database
    await reader.log.listening();
    await revise(1);
    await reader.log.position();
    const notes = async () => (await reader.select().from(note)).map((row) => row.id);
    const before = await notes();

    // commit a note through the writer unannounced, then a revision through the reader
    await writer.transaction((transaction) => transaction.insert(note).values(first).execute());
    await revise(2);

    // read the position, which finds the writer's commit in the log, then the notes anew
    await reader.log.position();
    expect({ before, after: await notes() }).toEqual({ before: [], after: [first.id] });
});

test("rerun a transaction once over an unannounced commit to a row it read, then commit on the fresh row", async () => {
    // open a file twice: a reader on a channel, and a writer that announces nothing
    const directory = await HOST.files.makeTemporaryDirectory("destack-sqlite-unheard-race-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const hub = channelHub<unknown>();
    const reader = await connectBunSqlite(path, changeTables, {
        environment: HOST,
        openChannel: () => hub(),
    });
    onTestFinished(() => reader.close());
    await reader.log.create("inbox");
    await reader.migrate(changeTables);
    const writer = await connectBunSqlite(path, changeTables, { environment: HOST });
    onTestFinished(() => writer.close());

    // store a note, hear commits, catch up, and read the position into memory
    await reader.transaction((transaction) => transaction.insert(note).values(first).execute());
    await reader.log.listening();
    await reader.log.position();
    await reader.log.position();

    // retitle the note through the writer, unannounced
    await writer.transaction((transaction) =>
        transaction.update(note).set({ title: "Second" }).where(eq(note.id, first.id)).execute(),
    );

    // append to the title read in a transaction, which loses one race and reads the fresh row
    const lost = reader.state.lostRaces;
    await Transaction.run(reader, async (transaction) => {
        const row = present(
            await transaction.read(note).findFirst({ where: { id: first.id } }),
            "the note",
        );
        await transaction.write({
            table: note,
            before: row,
            after: { ...row, title: `${row.title}!` },
        });
    });
    const [stored] = await reader.select({ title: note.title }).from(note);
    expect({ races: reader.state.lostRaces - lost, title: stored?.title }).toEqual({
        races: 1,
        title: "Second!",
    });
});
