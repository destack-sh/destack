import { testSimulation } from "@destack/test/world";
import { Digest, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { MemoryBlobStorage } from "./memory.ts";
import type { BlobPlacement } from "./provider.ts";

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

test("store blobs in packs, read each by its placement, key a lone blob by its digest, delete packs twice alike, and refuse other bytes", async () => {
    // store two blobs in one pack, and one alone
    const { crypto } = testSimulation().environment("test");
    const storage = new MemoryBlobStorage(crypto);
    const entry = async (text: string) => {
        const bytes = new TextEncoder().encode(text);

        return {
            digest: await Digest.of(bytes, crypto),
            size: bytes.byteLength,
            body: bytesOf(text),
        };
    };
    const [first, second] = await storage.write([await entry("first"), await entry("second")]);
    const [alone] = await storage.write([await entry("alone")]);
    const placed = present(alone, "the lone blob's placement");

    // delete the pack twice, then refuse reading it and refuse bytes of another digest
    const pack = present(first, "the first placement").pack;
    const read = (placement: BlobPlacement | undefined) => {
        const { pack: key, offset, size } = present(placement, "a placement");

        return textOf(storage.slice(key, offset, size));
    };
    const texts = [await read(first), await read(second)];
    await storage.delete(pack);
    await storage.delete(pack);
    const other = await entry("other");
    expect({
        texts,
        isPacked: first?.pack === second?.pack && pack !== first?.digest,
        alone: [placed.pack === placed.digest, await read(alone)],
        packs: await Array.fromAsync(storage.packs()),
        deleted: await refusal(read(first)),
        mismatch: await refusal(storage.write([{ ...other, body: bytesOf("first") }])),
    }).toEqual({
        texts: ["first", "second"],
        isPacked: true,
        alone: [true, "alone"],
        packs: [placed.pack],
        deleted: ["NOT_FOUND", "BLOB_NOT_FOUND", `pack '${pack}' not found`],
        mismatch: [
            "BAD_REQUEST",
            "DIGEST_INVALID",
            `blob '${(await entry("first")).digest}' is invalid: its digest must be '${other.digest}'`,
        ],
    });
});
