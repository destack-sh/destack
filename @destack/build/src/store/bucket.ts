import type { Bucket } from "@destack/bucket";
import type { Crypto } from "@destack/environment";
import { BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import type { Digest } from "@destack/schema";
import { BuildError } from "../error/index.ts";

/** The key prefix of the builds in a bucket. */
const BUILD_PREFIX = "build/";

/** Builds in a bucket, each a build directory under its manifest's digest. */
export class BuildBucket {
    /** The bucket keeping the builds. */
    readonly bucket: Bucket;
    /** The crypto verifying the builds' files. */
    readonly crypto: Crypto;

    /** Keep builds in a bucket. */
    constructor(bucket: Bucket, crypto: Crypto) {
        this.bucket = bucket;
        this.crypto = crypto;
    }

    /** Write a build unless kept, its manifest last so a build is readable only once whole. */
    async write(reader: BuildReader, manifest: Digest): Promise<void> {
        // leave a build written before, as a digest names one build
        const directory = `${BUILD_PREFIX}${manifest}/`;
        if ((await this.bucket.head(`${directory}${MANIFEST_PATH}`)) !== null) {
            return;
        }

        // write each file, then the manifest
        for (const file of await reader.distributed()) {
            await this.bucket.put(`${directory}${file.path}`, await reader.load(file.path));
        }
        await this.bucket.put(`${directory}${MANIFEST_PATH}`, JSON.stringify(reader.manifest));
    }

    /** Open a kept build, streaming its files from the bucket. */
    async read(manifest: Digest): Promise<BuildReader> {
        // read the manifest, refusing a build the bucket lacks
        const directory = `${BUILD_PREFIX}${manifest}/`;
        const found = await this.bucket.get(`${directory}${MANIFEST_PATH}`);
        if (found === null) {
            throw BuildError.of("BUILD_NOT_IN_BUCKET", `the bucket has no build ${manifest}`);
        }
        const parsed = PackageManifest.parse(JSON.parse(await found.text()));

        return new BuildReader(
            parsed,
            async (path) => {
                const file = await this.bucket.get(`${directory}${path}`);
                if (file === null) {
                    throw BuildError.of(
                        "BUILD_NOT_IN_BUCKET",
                        `the bucket has no ${path} of ${manifest}`,
                    );
                }

                return file.body;
            },
            this.crypto,
        );
    }
}
