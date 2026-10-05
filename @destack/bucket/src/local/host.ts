import { rm, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { Keychain } from "@destack/host/keychain";
import { schema } from "@destack/schema";
import { CatalogueBucketHost } from "../catalogue/host.ts";
import type { BucketReference } from "../s3/index.ts";
import { S3Credentials } from "../s3/index.ts";
import { LocalBucket } from "./bucket.ts";
import { syncDirectory } from "./directory.ts";

/** The random bytes of an access key identifier, twenty hexadecimal digits as AWS's are twenty characters. */
const ACCESS_KEY_BYTES = 10;

/** The random bytes of a secret access key, forty base64 characters as AWS's are. */
const SECRET_KEY_BYTES = 30;

/** The file in a bucket's directory marking it fenced while a transfer copies it. */
const FENCE_FILE = "fence";

/** Where a host serves its buckets over S3, and the credentials presigning their transfers. */
export interface LocalBucketHostOptions {
    /** The directory with one directory per bucket. */
    readonly directory: string;
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** The host's S3 credentials, which never leave it. */
    readonly credentials: S3Credentials;
}

/** A device host keeping buckets: one directory each, opened once, and served over S3 at one endpoint. */
export class LocalBucketHost extends CatalogueBucketHost<LocalBucket> {
    /** The provider code of local buckets. */
    readonly provider: string;
    /** The directory with one directory per bucket. */
    readonly directory: string;

    /** Keep a host's buckets under a directory. */
    constructor(options: LocalBucketHostOptions) {
        super(options);
        this.provider = "local";
        this.directory = options.directory;
    }

    /** Read the S3 credentials a keychain keeps under a name, generating them once and adopting those a racing writer kept first. */
    static async credentials(keychain: Keychain, name: string): Promise<S3Credentials> {
        // keep generated credentials when the keychain keeps none
        const kept = await Keychain.update(
            keychain,
            name,
            (credentials) => credentials ?? JSON.stringify(LocalBucketHost.#generate()),
        );

        return S3Credentials.parse(JSON.parse(kept));
    }

    /** Generate S3 credentials shaped as AWS's are. */
    static #generate(): S3Credentials {
        return {
            accessKeyId: crypto
                .getRandomValues(new Uint8Array(ACCESS_KEY_BYTES))
                .toHex()
                .toUpperCase(),
            secretAccessKey: crypto.getRandomValues(new Uint8Array(SECRET_KEY_BYTES)).toBase64(),
        };
    }

    /** Build the directory of a bucket. */
    path(bucket: Pick<BucketReference, "bucketId">): string {
        return join(this.directory, bucket.bucketId);
    }

    /** Name where a bucket lives: its directory's file URL. */
    override reference(bucket: Pick<BucketReference, "bucketId">): string {
        return pathToFileURL(this.path(bucket)).href;
    }

    /** Open the bucket an S3 request addresses by its resource, absent when the host has none. */
    async named(name: string): Promise<LocalBucket | undefined> {
        // refuse a name no bucket takes, and a bucket without a directory
        const bucketId = schema.identifier("bucket").safeParse(name);
        if (!bucketId.success) {
            return undefined;
        }
        const isPresent = await exists(this.path({ bucketId: bucketId.data }));

        return isPresent ? this.open({ bucketId: bucketId.data }) : undefined;
    }

    /** Fence a bucket while a transfer copies it, across restarts until lifted, returning once its writes in flight finished, and report whether this call set the fence. */
    override async fence(bucket: Pick<BucketReference, "bucketId">): Promise<boolean> {
        // mark the bucket fenced in its directory, then refuse its writes
        const opened = await this.open(bucket);
        await writeFile(join(this.path(bucket), FENCE_FILE), "");
        await syncDirectory(this.path(bucket));

        return await opened.fence();
    }

    /** Lift a bucket's fence, accepting its writes again. */
    override async lift(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        // remove the fence file, then accept the bucket's writes
        const opened = await this.open(bucket);
        await rm(join(this.path(bucket), FENCE_FILE), { force: true });
        await syncDirectory(this.path(bucket));
        await opened.lift();
    }

    /** Close a bucket and remove its directory with its files. */
    override async destroy(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        await this.close(bucket);
        await rm(this.path(bucket), { recursive: true, force: true });
    }

    /** Open a bucket's directory, fenced when its fence file remains from a transfer. */
    protected override async load(
        bucket: Pick<BucketReference, "bucketId">,
        scope: string | undefined,
    ): Promise<LocalBucket> {
        // read whether a transfer fenced the bucket, then open it
        const isFenced = await exists(join(this.path(bucket), FENCE_FILE));
        const opened = await LocalBucket.open(this.path(bucket), scope);

        // refuse writes while the fence file remains
        if (isFenced) {
            await opened.fence();
        }

        return opened;
    }
}

/** Report whether a path exists. */
async function exists(path: string): Promise<boolean> {
    return await stat(path).then(
        () => true,
        (error: NodeJS.ErrnoException) => {
            if (error.code === "ENOENT") {
                return false;
            }
            throw error;
        },
    );
}
