import { BLOB_TABLES, bucket, defineTable, integer, json } from "@destack/db";
import { BlobStore } from "@destack/db/database";
import { TestDatabase } from "@destack/db/test";
import { MemoryBlobStorage } from "@destack/resource";
import { Digest } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { HmacKey } from "../webhook/hmac.ts";
import { ObjectFileServer, type ObjectFileServerOptions, ObjectFile } from "./index.ts";

/** Rows that have a file. */
const row = defineTable("row", {
    /** The row. */
    id: integer("id").primaryKey(),
    /** The file. */
    file: json("file", ObjectFile, { blobs: "parts" }),
});

/** Serve files over memory storage. */
async function openFiles(rate?: ObjectFileServerOptions["rate"]) {
    const simulation = testSimulation();
    const environment = simulation.environment("space");
    const opened = await TestDatabase.create([row, bucket, ...BLOB_TABLES], {
        environment,
        isMigrated: true,
    });
    onTestFinished(() => opened.close());
    const { database } = opened;
    const blobs = new BlobStore({
        storage: new MemoryBlobStorage(environment.crypto),
        database,
        environment,
    });
    const key = await HmacKey.of(new Uint8Array(32).fill(1), environment.crypto);
    const files = new ObjectFileServer({
        blobs,
        database,
        environment,
        key: async () => key,
        types: [],
        read: async () => undefined,
        ...(rate === undefined ? {} : { rate }),
    });
    const send = async (caller: string, text: string) => {
        const bytes = new TextEncoder().encode(text);

        return files.send(caller, await Digest.of(bytes, environment.crypto), chunk(text));
    };

    return { simulation, environment, database, blobs, files, key, send };
}

/** Read the text of some bytes. */
async function textOf(body: AsyncIterable<Uint8Array>): Promise<string> {
    const decoder = new TextDecoder();
    let text = "";
    for await (const bytes of body) {
        text += decoder.decode(bytes, { stream: true });
    }

    return text;
}

/** Read text as one chunk of bytes. */
async function* chunk(text: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(text);
}

test("make a file of a part a caller sent, and read it whole and by range", async () => {
    const { environment, files, key, send } = await openFiles();
    const part = await send("user-a", "hello file");
    const file = await files.complete("user-a", {
        parts: [part],
        name: "note.txt",
        type: "text/plain",
    });
    const { signature, ...shown } = file;
    const other = await HmacKey.of(new Uint8Array(32).fill(2), environment.crypto);

    expect({
        shown,
        isSigned: [
            await ObjectFile.verify(file, key),
            await ObjectFile.verify({ ...file, name: "b.txt" }, key),
            await ObjectFile.verify({ ...file, signature: part.signature }, key),
            await ObjectFile.verify(file, other),
        ],
        whole: await textOf(files.read(file)),
        range: await textOf(files.read(file, { offset: 6, length: 100 })),
    }).toEqual({
        shown: {
            parts: [part.digest],
            size: 10,
            name: "note.txt",
            type: "text/plain",
            picture: null,
        },
        isSigned: [true, false, false, false],
        whole: "hello file",
        range: "file",
    });
});

test("refuse other bytes, another caller's part, a short part, and a part past its day", async () => {
    const { simulation, files, send } = await openFiles();
    const part = await send("user-a", "mine");
    const input = { parts: [part], name: "a.txt", type: "text/plain" };
    const other = await refusal(files.send("user-b", part.digest, chunk("guessed")));
    const another = await refusal(files.complete("user-b", input));
    const longer = await refusal(files.complete("user-a", { ...input, parts: [part, part] }));
    const empty = await refusal(send("user-a", ""));
    await simulation.jump(simulation.clock.now() + 24 * 60 * 60_000);

    expect({
        other,
        another,
        longer,
        empty,
        expired: await refusal(files.complete("user-a", input)),
    }).toEqual({
        other: ["BAD_REQUEST", "PART_INVALID", "part is invalid: its bytes have another digest"],
        another: ["BAD_REQUEST", "PART_INVALID", "part is invalid: the caller did not send it"],
        longer: [
            "BAD_REQUEST",
            "PART_INVALID",
            "part is invalid: only the last part of a file is shorter",
        ],
        empty: ["BAD_REQUEST", "PART_INVALID", "part is invalid: it is empty"],
        expired: ["BAD_REQUEST", "PART_INVALID", "part is invalid: it expired, so send it again"],
    });
});

test("keep a file's parts while a row has it, and forget parts no file came of", async () => {
    const { simulation, files, blobs, database, send } = await openFiles();
    const kept = await send("user-a", "kept");
    const left = await send("user-a", "left");
    const file = await files.complete("user-a", {
        parts: [kept],
        name: "kept.txt",
        type: "text/plain",
    });
    await database.transaction(async (transaction) => {
        await transaction.insert(row).values({ id: 1, file });
    });
    await simulation.jump(simulation.clock.now() + 25 * 60 * 60_000);
    await blobs.collect();
    await simulation.jump(simulation.clock.now() + 2 * 60 * 60_000);
    await blobs.collect();

    expect([...(await blobs.missing([kept.digest, left.digest]))]).toEqual([left.digest]);
});

test("refuse parts past a caller's rate, saying when the next is due", async () => {
    const { send } = await openFiles({ count: 2, every: { minutes: 1 } });
    await send("user-a", "one");
    await send("user-a", "two");

    expect({
        refused: await refusal(send("user-a", "three")),
        other: (await send("user-b", "three")).size,
    }).toEqual({
        refused: [
            "TOO_MANY_REQUESTS",
            "RATE_EXHAUSTED",
            "caller 'user-a' exceeds its limit of 2 parts per 60000 ms",
        ],
        other: 5,
    });
});
