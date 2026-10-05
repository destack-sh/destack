import { Updater } from "tuf-js";
import { Metadata, MetadataKind } from "@tufjs/models";
import { isDeepStrictEqual } from "node:util";
import { Readable } from "node:stream";
import { mkdir, mkdtemp, readFile, rm, writeFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { RepositoryConfiguration } from "./index.ts";
import { print } from "../output/index.ts";
import { parseDocument } from "./document.ts";

/** Source checkout containing the embedded update trust root. */
const root = fileURLToPath(new URL("../../../../", import.meta.url));
/** Signed repository prepared by this release or renewal. */
const directory = resolve(root, process.argv[2] ?? "dist/update");
/** Temporary verifier state, independent of the signing job. */
const cache = await mkdtemp(join(tmpdir(), "destack-verify-"));
/** Selected release repository. */
const configuration = new RepositoryConfiguration();

try {
    // authenticate public metadata starting at the root shipped to clients
    const metadata = join(cache, "metadata");
    await mkdir(metadata);
    await writeFile(
        join(metadata, "root.json"),
        JSON.stringify((await configuration.root()).toJSON()),
    );
    const updater = new Updater({
        metadataDir: metadata,
        metadataBaseUrl: new URL("metadata/", configuration.url).href,
        targetDir: join(cache, "targets"),
        targetBaseUrl: new URL("targets/", configuration.url).href,
    });
    await updater.refresh();

    // require the exact signed timestamp produced by this job
    const expected = await readFile(join(directory, "metadata/timestamp.json"));
    const actual = await readFile(join(metadata, "timestamp.json"));
    const expectedDocument = parseDocument(expected.toString());
    const actualDocument = parseDocument(actual.toString());
    if (!isDeepStrictEqual(expectedDocument, actualDocument)) {
        throw new Error("published update revision differs from the signed release");
    }

    // verify newly uploaded archives with bounded memory
    const targets = Metadata.fromJSON(
        MetadataKind.Targets,
        parseDocument(await readFile(join(metadata, "targets.json"), "utf8")),
    );
    for (const target of Object.values(targets.signed.targets)) {
        const path = `${target.hashes["sha256"]}.${target.path}`;
        try {
            await stat(join(directory, "targets", path));
        } catch (error) {
            if (error instanceof Error && "code" in error && error.code === "ENOENT") {
                continue;
            }
            throw error;
        }
        const response = await fetch(new URL(`targets/${path}`, configuration.url), {
            redirect: "error",
            signal: AbortSignal.timeout(120000),
        });
        if (!response.ok || !response.body) {
            throw new Error(`cannot verify published target: ${path} (${response.status})`);
        }
        await target.verify(Readable.from(response.body));
    }
    const revision = Metadata.fromJSON(MetadataKind.Timestamp, actualDocument).signed.version;
    print(`Verified published update revision ${revision}.`);
} finally {
    await rm(cache, { recursive: true, force: true });
}
