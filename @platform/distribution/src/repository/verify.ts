import { CHECKOUT } from "../distribution/distribution.ts";
import { Updater } from "tuf-js";
import { Metadata, MetadataKind, type TargetFile } from "@tufjs/models";
import { isDeepStrictEqual } from "node:util";
import { Readable } from "node:stream";
import { join, resolve } from "node:path";
import { type Environment, FileSystemError, type HostCapability } from "@destack/environment";
import { RepositoryConfiguration } from "./index.ts";
import { print } from "../distribution/output.ts";
import { MetadataDocument } from "./document.ts";

/** How long downloading one published target may take, in milliseconds. */
const DOWNLOAD_TIMEOUT_MILLISECONDS = 120_000;

/** Verify the published repository against the one this release or renewal signed. */
export async function verifyUpdate(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the signed repository, a temporary verifier state and the release repository
    const { files } = environment;
    const [selected = "dist/update"] = arguments_;
    const directory = resolve(CHECKOUT, selected);
    const cache = await files.makeTemporaryDirectory("destack-verify-");
    const configuration = new RepositoryConfiguration(environment);
    try {
        // authenticate public metadata starting at the root shipped to clients
        const metadata = join(cache, "metadata");
        await files.mkdir(metadata);
        await files.writeTextFile(
            join(metadata, "root.json"),
            JSON.stringify(configuration.root().toJSON()),
        );
        const updater = new Updater({
            metadataDir: metadata,
            metadataBaseUrl: new URL("metadata/", configuration.url).href,
            targetDir: join(cache, "targets"),
            targetBaseUrl: new URL("targets/", configuration.url).href,
        });
        await updater.refresh();

        // require the exact signed timestamp produced by this job
        const expected = await files.readTextFile(join(directory, "metadata/timestamp.json"));
        const actual = await files.readTextFile(join(metadata, "timestamp.json"));
        const expectedDocument = MetadataDocument.parse(expected);
        const actualDocument = MetadataDocument.parse(actual);
        if (!isDeepStrictEqual(expectedDocument, actualDocument)) {
            throw new Error("published update revision differs from the signed release");
        }

        // verify newly uploaded archives with bounded memory
        const targets = Metadata.fromJSON(
            MetadataKind.Targets,
            MetadataDocument.parse(await files.readTextFile(join(metadata, "targets.json"))),
        );
        for (const target of Object.values(targets.signed.targets)) {
            await verifyTarget(directory, target, configuration, environment);
        }
        const revision = Metadata.fromJSON(MetadataKind.Timestamp, actualDocument).signed.version;
        print(`Verified published update revision ${revision}.`, environment);
    } finally {
        await files.remove(cache, { recursive: true, force: true });
    }
}

/** Download a target this job uploaded and verify its bytes, leaving one it did not upload. */
async function verifyTarget(
    directory: string,
    target: TargetFile,
    configuration: RepositoryConfiguration,
    environment: Environment<HostCapability>,
): Promise<void> {
    // leave a target this job did not upload
    const path = `${target.hashes["sha256"]}.${target.path}`;
    const uploaded = await environment.files
        .stat(join(directory, "targets", path))
        .catch((error: unknown) => FileSystemError.absent(error));
    if (uploaded === undefined) {
        return;
    }

    // download the published target and verify its bytes
    const response = await environment.network.fetch(
        new Request(new URL(`targets/${path}`, configuration.url), {
            redirect: "error",
            signal: environment.clock.timeout(DOWNLOAD_TIMEOUT_MILLISECONDS),
        }),
    );
    if (!response.ok || !response.body) {
        throw new Error(`cannot verify published target: '${path}' (${response.status})`);
    }
    await target.verify(Readable.from(response.body));
}
