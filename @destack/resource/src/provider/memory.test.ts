import { testSimulation } from "@destack/test/world";
import { Digest } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { MemoryBlobStorage } from "./memory.ts";

/** Yield some text as one chunk of bytes. */
async function* bytesOf(text: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(text);
}

/** Read a blob's chunks as text. */
async function textOf(chunks: AsyncIterable<Uint8Array>): Promise<string> {
    const read = await Array.fromAsync(chunks);

    return new TextDecoder().decode(
        await new Blob(read.map((chunk) => new Uint8Array(chunk))).bytes(),
    );
}

test("keep blobs by digest, read and slice them, copy the missing ones, and delete them twice alike", async () => {
    // keep a blob, then copy it and another into a second storage
    const { crypto } = testSimulation().environment("test");
    const source = new MemoryBlobStorage(crypto);
    const kept = await source.write(bytesOf("kept bytes"));
    const other = await source.write(bytesOf("other"));
    const target = new MemoryBlobStorage(crypto);
    await target.write(bytesOf("kept bytes"));
    const lacking = await target.missing([kept, other]);
    await target.fetch([kept, other], source);

    // delete one twice, then refuse reading it and refuse bytes of another digest
    await target.delete(other);
    await target.delete(other);
    const absent = await Digest.of(new TextEncoder().encode("absent"), crypto);
    expect({
        lacking,
        read: await textOf(target.read(kept)),
        slice: await textOf(target.slice(kept, 5, 3)),
        digests: await Array.fromAsync(target.digests()),
        deleted: await refusal(textOf(target.read(other))),
        mismatch: await refusal(target.write(bytesOf("kept bytes"), absent)),
    }).toEqual({
        lacking: [other],
        read: "kept bytes",
        slice: "byt",
        digests: [kept],
        deleted: ["NOT_FOUND", "BLOB_NOT_FOUND", `blob ${other} not found`],
        mismatch: ["BAD_REQUEST", "BAD_DIGEST", `expected blob ${absent}, got ${kept}`],
    });
});
