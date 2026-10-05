import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Key, Metadata, Root } from "@tufjs/models";
import { Updater, type UpdaterOptions } from "tuf-js";
import { HardwareKey } from "../yubico/hardware.ts";
import {
    type Distribution,
    readPublicKey,
    type ReleaseKeys,
    SignedRepository,
    SigningKey,
    TrustedRoot,
} from "@destack/update/publish";
import { schema } from "@destack/schema";
import { print } from "../output/index.ts";
import { readCommit } from "../distribution/distribution.ts";

/** The version a client's trusted root document carries. */
const TrustedVersion = schema
    .object({ signed: schema.object({ version: schema.number() }).strip() })
    .strip();

/** Exercise hardware signing and root rotation through the real TUF client on loopback. */
async function rehearse(): Promise<void> {
    // select the rehearsal hardware and generate disposable keys for every other role
    const hardware = await readHardware();
    const [first, second, third] = [
        SigningKey.generate(),
        SigningKey.generate(),
        SigningKey.generate(),
    ];
    const keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    const expires = new Date(Date.now() + 365 * 86_400_000).toISOString();

    // serve the temporary repository on loopback
    const directory = await mkdtemp(join(tmpdir(), "destack-signing-rehearsal-"));
    const repository = join(directory, "repository");
    const server = serveRepository(repository);
    try {
        // create an initial two-of-three root using one real hardware signature
        const roots = [hardware.public, first.public, second.public];
        const root = signRoot(1, roots, keys, expires, hardware, [first]);
        TrustedRoot.verify(root);
        const content = Buffer.from("Destack signing rehearsal\n");
        const distribution = await writeArtifact(directory, content);
        await SignedRepository.create(repository, 1, root, keys, distribution);

        // authenticate metadata and download a digest-verified target through tuf-js
        const options = await createClientOptions(directory, server.url, root);
        await verifyTarget(options, content);

        // replace a root key and require authorization from both old and new key sets
        const rotatedRoots = [hardware.public, second.public, third.public];
        const rotated = signRoot(2, rotatedRoots, keys, expires, hardware, [first, second]);
        TrustedRoot.verify(rotated, root);
        await SignedRepository.create(repository, 2, rotated, keys, distribution);
        await verifyRotation(options);
        print("hardware RSA-PSS signing, target verification and root rotation passed");
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
}

/** Select the rehearsal hardware without loading any production credential. */
async function readHardware(): Promise<HardwareKey> {
    // require the public key, serial and PKCS#11 module arguments
    const [publicPath, serial, module, ...extra] = process.argv.slice(2);
    if (
        publicPath === undefined ||
        serial === undefined ||
        module === undefined ||
        extra.length > 0
    ) {
        throw new Error("usage: rehearse.ts <public-key.pem> <serial> <pkcs11-module>");
    }

    return new HardwareKey(readPublicKey(await readFile(publicPath, "utf8")), serial, module);
}

/** Serve the metadata and target files of a repository directory on a loopback port. */
function serveRepository(repository: string): Bun.Server<undefined> {
    return Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            // expose only the temporary repository's public files
            const path = new URL(request.url).pathname;
            if (!/^\/(metadata|targets)\/[^/]+$/u.test(path)) {
                return new Response(null, { status: 404 });
            }
            const file = Bun.file(join(repository, path));

            return (await file.exists()) ? new Response(file) : new Response(null, { status: 404 });
        },
    });
}

/** Create a root signed by the hardware key and the disposable signers. */
function signRoot(
    version: number,
    roots: Key[],
    keys: ReleaseKeys,
    expires: string,
    hardware: HardwareKey,
    signers: SigningKey[],
): Metadata<Root> {
    // create the root over the online role keys
    const publicKeys = {
        targets: keys.targets.public,
        snapshot: keys.snapshot.public,
        timestamp: keys.timestamp.public,
    };
    const root = TrustedRoot.create(version, roots, publicKeys, expires);

    // sign with the YubiKey and check its signature before the disposable signatures
    print(`sign rehearsal root ${version} with the selected YubiKey`);
    root.sign((bytes) => hardware.sign(bytes));
    hardware.public.verifySignature(root);
    for (const signer of signers) {
        root.sign((bytes) => signer.sign(bytes));
    }

    return root;
}

/** Write the rehearsal artifact and describe it as the one distribution to publish. */
async function writeArtifact(directory: string, content: Buffer): Promise<Distribution[]> {
    // write the content as the archive of an arm64 macOS release
    const archive = join(directory, "artifact");
    await writeFile(archive, content);
    const commit = await readCommit(import.meta.dirname);

    return [{ target: "aarch64-apple-darwin", version: "2026.9.1", commit, archive }];
}

/** Pin a root in a fresh client metadata directory and return the client options for the server. */
async function createClientOptions(
    directory: string,
    url: URL,
    root: Metadata<Root>,
): Promise<UpdaterOptions> {
    // pin the root as the client's trusted root
    const metadata = join(directory, "client");
    await mkdir(metadata);
    await writeFile(join(metadata, "root.json"), SignedRepository.encode(root));

    // download targets into their own directory
    const download = join(directory, "download");
    const options = {
        metadataDir: metadata,
        metadataBaseUrl: new URL("/metadata/", url).href,
        targetDir: download,
        targetBaseUrl: new URL("/targets/", url).href,
    };
    await mkdir(download);

    return options;
}

/** Refresh the client and require the downloaded target to equal the signed content. */
async function verifyTarget(options: UpdaterOptions, content: Buffer): Promise<void> {
    // download the target through the authenticated metadata
    const client = new Updater(options);
    await client.refresh();
    const target = await client.getTargetInfo("aarch64-apple-darwin.tar.gz");
    if (!target || !(await readFile(await client.downloadTarget(target))).equals(content)) {
        throw new Error("rehearsal target differs from its signed content");
    }
}

/** Refresh a new client and require it to trust root 2. */
async function verifyRotation(options: UpdaterOptions): Promise<void> {
    // read the trusted root the client persisted
    const updated = new Updater(options);
    await updated.refresh();
    const trusted = TrustedVersion.parse(
        JSON.parse(await readFile(join(options.metadataDir, "root.json"), "utf8")),
    );
    if (trusted.signed.version !== 2) {
        throw new Error("client did not accept the root rotation");
    }
}

await rehearse();
