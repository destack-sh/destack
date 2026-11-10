import { createHash } from "node:crypto";
import type { Random } from "@destack/environment";
import { BlobPack, type BlobBody, type BlobPlacement, type BlobStorage } from "@destack/resource";
import { Digest, present, Uuid } from "@destack/schema";
import { type Bucket, type UploadedPart, MAX_BATCH_FILES } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";

/** The bytes of each part of a blob written in parts: 8 MiB. */
const PART_BYTES = 8 * 1024 * 1024;

/** Blobs in packs, files of any bucket below a prefix, such as an R2 or S3 bucket's. */
export class BucketBlobStorage implements BlobStorage {
    /** The residency's bucket. */
    readonly #files: Bucket;
    /** The prefix of the bucket's packs, such as `bucket-…/`. */
    readonly #prefix: string;
    /** The randomness naming each pack of several blobs. */
    readonly #random: Random;

    /** Store a bucket's blobs below a prefix of the residency's bucket. */
    constructor(files: Bucket, prefix: string, random: Random) {
        this.#files = files;
        this.#prefix = prefix;
        this.#random = random;
    }

    /** Store the bodies in one pack, uploaded in parts straight to its key, each body verified. */
    async write(bodies: readonly BlobBody[]): Promise<readonly BlobPlacement[]> {
        // leave a lone blob of known digest stored before
        const pack = BlobPack.key(bodies, () => Uuid.v4(this.#random));
        const key = this.#key(pack);
        const [only] = bodies;
        const known =
            bodies.length === 1 && only?.digest !== undefined && only.size !== undefined
                ? { digest: only.digest, size: only.size }
                : undefined;
        if (bodies.length === 0) {
            return [];
        } else if (known !== undefined && (await this.#files.head(key)) !== null) {
            return BlobPack.place(pack, [known]);
        }

        // read up to two parts to tell a pack of one part from a longer one, placing each blob read
        const blobs: { readonly digest: Digest; readonly size: number }[] = [];
        const parts = partsOf(verifiedBodies(bodies, (blob) => blobs.push(blob)));
        const first = await parts.next();
        const second = await parts.next();

        // store a pack of one part at once, a lone blob of known digest under its checksum
        if (first.done === true || second.done === true) {
            const bytes = first.done === true ? new Uint8Array() : first.value;
            await this.#files.put(key, bytes, known === undefined ? {} : { sha256: known.digest });
        }
        // upload a longer pack in parts
        else {
            await this.#upload(key, resumed([first.value, second.value], parts));
        }

        return BlobPack.place(pack, blobs);
    }

    /** Read a range of a pack's bytes. */
    async *slice(pack: string, offset: number, length: number): AsyncIterable<Uint8Array> {
        const file = await this.#files.get(this.#key(pack), { range: { offset, length } });
        yield* present(file, pack).body;
    }

    /** Delete a pack. */
    async delete(pack: string): Promise<void> {
        await this.#files.delete(this.#key(pack));
    }

    /** List the packs directly below the prefix. */
    async *packs(): AsyncIterable<string> {
        for await (const keys of this.#pages(this.#prefix, "/")) {
            for (const key of keys) {
                const name = key.slice(this.#prefix.length);
                if (BlobPack.isKey(name)) {
                    yield name;
                }
            }
        }
    }

    /** Clean up after interrupted writes, whose multipart uploads the bucket's lifecycle aborts. */
    async clean(): Promise<void> {}

    /** Delete every file below the prefix, such as when the bucket is destroyed. */
    async clear(): Promise<void> {
        for await (const keys of this.#pages(this.#prefix)) {
            await this.#files.delete(keys);
        }
    }

    /** Upload a body in parts of one size under a key, skipping an empty last part. */
    async #upload(key: string, parts: AsyncIterable<Uint8Array<ArrayBuffer>>): Promise<void> {
        const upload = await this.#files.createMultipartUpload(key);
        try {
            const uploaded: UploadedPart[] = [];
            for await (const part of parts) {
                if (part.byteLength > 0) {
                    uploaded.push(await upload.uploadPart(uploaded.length + 1, part));
                }
            }
            await upload.complete(uploaded);
        } catch (error) {
            // abort the upload, keeping the failure beside a failure to abort
            try {
                await upload.abort();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "pack upload and abort failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }
    }

    /** List the keys below a prefix a page at a time. */
    async *#pages(prefix: string, delimiter?: string): AsyncIterable<string[]> {
        let cursor: string | undefined;
        do {
            const page = await this.#files.list({
                prefix,
                limit: MAX_BATCH_FILES,
                ...(delimiter === undefined ? {} : { delimiter }),
                ...(cursor === undefined ? {} : { cursor }),
            });
            if (page.files.length > 0) {
                yield page.files.map((file) => file.key);
            }
            cursor = page.cursor;
        } while (cursor !== undefined);
    }

    /** Name a pack's file. */
    #key(pack: string): string {
        if (!BlobPack.isKey(pack)) {
            throw BucketError.of("INVALID_ARGUMENT", `not a pack key: ${pack}`);
        }

        return `${this.#prefix}${pack}`;
    }
}

/** Pass on bodies' bytes in order, digesting and counting each, refusing an unexpected one. */
async function* verifiedBodies(
    bodies: readonly BlobBody[],
    placed: (blob: { readonly digest: Digest; readonly size: number }) => void,
): AsyncIterable<Uint8Array> {
    for (const body of bodies) {
        // hash and count the body's chunks on the way
        const hash = createHash("sha256");
        let size = 0;
        for await (const chunk of body.body) {
            hash.update(chunk);
            size += chunk.byteLength;
            yield chunk;
        }

        // refuse bytes unlike the expected digest or size, and report the placed blob
        const digest = Digest.parse(hash.digest("hex"));
        BlobPack.verify(body, digest, size);
        placed({ digest, size });
    }
}

/** Split a body into parts of one size and a last shorter or empty one. */
async function* partsOf(
    body: AsyncIterable<Uint8Array>,
): AsyncGenerator<Uint8Array<ArrayBuffer>, void> {
    // buffer chunks until they fill a part
    let buffered: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of body) {
        // pass on each full part
        let rest = chunk;
        while (size + rest.byteLength >= PART_BYTES) {
            const taken = PART_BYTES - size;
            buffered.push(rest.subarray(0, taken));
            yield joined(buffered, PART_BYTES);
            buffered = [];
            size = 0;
            rest = rest.subarray(taken);
        }

        // buffer the rest for the next part
        if (rest.byteLength > 0) {
            buffered.push(rest);
            size += rest.byteLength;
        }
    }
    yield joined(buffered, size);
}

/** Pass on the parts read so far, then the rest. */
async function* resumed(
    read: readonly Uint8Array<ArrayBuffer>[],
    rest: AsyncGenerator<Uint8Array<ArrayBuffer>, void>,
): AsyncIterable<Uint8Array<ArrayBuffer>> {
    yield* read;
    yield* rest;
}

/** Join chunks into one array of their total size. */
function joined(chunks: readonly Uint8Array[], size: number): Uint8Array<ArrayBuffer> {
    // copy each chunk after the one before
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }

    return bytes;
}
