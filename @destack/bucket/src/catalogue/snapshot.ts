import { DatabaseHandle } from "@destack/db";
import type { BlobStorage } from "@destack/resource";
import { canonicalize, Digest, schema } from "@destack/schema";
import { part, segment } from "./stack/index.ts";

/** A snapshot of a bucket. */
const SnapshotManifest = schema.object({
    /** The digest of the catalogue's row snapshot. */
    catalogue: Digest,
    /** The digests of the blobs the catalogue's rows reference. */
    blobs: schema.array(Digest),
});

/** A snapshot's manifest. */
type SnapshotManifest = schema.Infer<typeof SnapshotManifest>;

/** A bucket opened for copying. */
export interface CatalogueHandle extends DatabaseHandle {
    /** The store of the blobs the catalogue's segments and parts reference. */
    readonly blobs: BlobStorage;
    /** Keep every blob from collection until the hold is released. */
    readonly hold: () => Promise<AsyncDisposable>;
    /** Collect the blobs deleted rows referenced, deleting those no row references any longer. */
    readonly retire: (digests: readonly Digest[]) => Promise<void>;
}

/** Copy a bucket's catalogue and blobs into a snapshot store. */
export const BucketSnapshot = {
    /** Copy the catalogue's rows and the blobs they reference into a store. */
    async take(
        bucket: CatalogueHandle,
        store: Pick<BlobStorage, "read" | "write" | "fetch">,
    ): Promise<Digest> {
        // copy the rows in one read, noting the blobs their segments and parts reference
        await using _held = await bucket.hold();
        const blobs = new Set<Digest>();
        const catalogue = await DatabaseHandle.snapshot(bucket, store, async (table, row) => {
            if (table === segment || table === part) {
                blobs.add(Digest.parse(row["blob"]));
            }

            return row;
        });

        // copy the referenced blobs beside the rows
        const referenced = [...blobs].toSorted();
        await store.fetch(referenced, bucket.blobs);

        return write(store, { catalogue, blobs: referenced });
    },

    /** Restore a copy into a bucket, its blobs before the rows that reference them. */
    async restore(
        bucket: CatalogueHandle,
        digest: Digest,
        store: Pick<BlobStorage, "read">,
        base?: Digest,
    ): Promise<void> {
        // keep the blobs the base lacks, held from collection until their rows commit
        const manifest = await read(store, digest);
        const held = base === undefined ? undefined : await read(store, base);
        {
            await using _held = await bucket.hold();
            const added = manifest.blobs.filter((blob) => held?.blobs.includes(blob) !== true);
            await bucket.blobs.fetch(added, store);
            await DatabaseHandle.restore(
                bucket,
                manifest.catalogue,
                store,
                undefined,
                held?.catalogue,
            );
        }

        // collect the blobs only the base referenced
        const released = held?.blobs.filter((blob) => !manifest.blobs.includes(blob)) ?? [];
        await bucket.retire(released);
    },
};

/** Keep a manifest in a store under its digest. */
async function write(
    store: Pick<BlobStorage, "write">,
    manifest: SnapshotManifest,
): Promise<Digest> {
    return store.write(chunked(new TextEncoder().encode(canonicalize(manifest))));
}

/** Read a manifest a store keeps. */
async function read(store: Pick<BlobStorage, "read">, digest: Digest): Promise<SnapshotManifest> {
    // decode the chunks as one text
    const decoder = new TextDecoder();
    let text = "";
    for await (const chunk of store.read(digest)) {
        text += decoder.decode(chunk, { stream: true });
    }

    return SnapshotManifest.parse(JSON.parse(text + decoder.decode()));
}

/** Yield bytes as one chunk. */
async function* chunked(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}
