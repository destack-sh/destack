import { type BlobStore, DatabaseHandle } from "@destack/db";
import type { ContentStore } from "@destack/resource";
import type {} from "@destack/package/import-meta";
import { canonicalize, Digest, schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { part, segment } from "./stack/index.ts";
import type { CatalogueEnvironment, CatalogueProvider } from "./storage.ts";

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
    /** The blobs the catalogue's segments and parts reference. */
    readonly blobs: BlobStore;
    /** The provider code of the host that opened the catalogue. */
    readonly provider: CatalogueProvider;
    /** The clock, randomness, network and telemetry the catalogue runs on. */
    readonly environment: CatalogueEnvironment;
}

/** Copy a bucket's catalogue and blobs into a snapshot store. */
export const BucketSnapshot = {
    /** Copy the catalogue's rows and the blobs they reference into a store. */
    take(
        bucket: CatalogueHandle,
        store: Pick<ContentStore, "read" | "write" | "fetch">,
    ): Promise<Digest> {
        return trace(bucket, "take", () => take(bucket, store));
    },

    /** Restore a copy into a bucket, its blobs before the rows that reference them. */
    restore(
        bucket: CatalogueHandle,
        digest: Digest,
        store: Pick<ContentStore, "read" | "batch">,
        base?: Digest,
    ): Promise<void> {
        return trace(bucket, "restore", () => restore(bucket, digest, store, base));
    },
};

/** Run a snapshot verb in its span on the bucket's telemetry, recording its duration. */
function trace<Value>(
    bucket: CatalogueHandle,
    verb: "take" | "restore",
    run: () => Promise<Value>,
): Promise<Value> {
    const provider = { "destack.bucket.provider": bucket.provider };

    return TelemetryScope.of(import.meta.destack.package, bucket.environment.telemetry).span(
        `destack.bucket.snapshot.${verb}`,
        provider,
        run,
        {
            duration: { attributes: provider },
        },
    );
}

/** Copy the catalogue's rows and the blobs they reference into a store. */
async function take(
    bucket: CatalogueHandle,
    store: Pick<ContentStore, "read" | "write" | "fetch">,
): Promise<Digest> {
    // copy the rows in one read, noting the blobs their segments and parts reference
    const blobs = new Set<Digest>();
    const catalogue = await DatabaseHandle.snapshot(bucket, store, async (table, row) => {
        if (table === segment || table === part) {
            blobs.add(Digest.parse(row["blob"]));
        }

        return row;
    });

    // copy the referenced blobs beside the rows, which a collection deletes only a grace after their last reference
    const referenced = [...blobs].toSorted();
    await store.fetch(referenced, bucket.blobs);

    return write(store, { catalogue, blobs: referenced });
}

/** Restore a copy into a bucket, its blobs before the rows that reference them. */
async function restore(
    bucket: CatalogueHandle,
    digest: Digest,
    store: Pick<ContentStore, "read" | "batch">,
    base: Digest | undefined,
): Promise<void> {
    // store the blobs the base lacks, leased until their rows commit
    const manifest = await read(store, digest);
    const held = base === undefined ? undefined : await read(store, base);
    const added = manifest.blobs.filter((blob) => held?.blobs.includes(blob) !== true);
    await bucket.blobs.fetch(added, store);
    await DatabaseHandle.restore(bucket, manifest.catalogue, store, undefined, held?.catalogue);

    // retire the blobs only the base referenced, whose rows the restore replaced
    const released = held?.blobs.filter((blob) => !manifest.blobs.includes(blob)) ?? [];
    await bucket.blobs.retire(released);
}

/** Keep a manifest in a store under its digest. */
async function write(
    store: Pick<ContentStore, "write">,
    manifest: SnapshotManifest,
): Promise<Digest> {
    return store.write(chunked(new TextEncoder().encode(canonicalize(manifest))));
}

/** Read a manifest a store keeps. */
async function read(store: Pick<ContentStore, "read">, digest: Digest): Promise<SnapshotManifest> {
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
