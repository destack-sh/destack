import {
    type Environment,
    FileSystemError,
    type HostCapability,
    type Listener,
} from "@destack/environment";
import { join } from "node:path";
import type { Key, Metadata, Root } from "@tufjs/models";
import { Updater, type UpdaterOptions } from "tuf-js";
import { HardwareKey } from "./hardware.ts";
import {
    type Distribution,
    readPublicKey,
    type ReleaseKeys,
    SignedRepository,
    SigningKey,
    TrustedRoot,
} from "@destack/update/publish";
import { schema } from "@destack/schema";
import { print } from "../distribution/output.ts";
import { readCommit } from "../distribution/distribution.ts";

/** The milliseconds a rehearsal root stays valid: one year. */
const ROOT_LIFETIME_MILLISECONDS = 365 * 86_400_000;

/** The version of a client's trusted root document. */
const TrustedVersion = schema
    .object({ signed: schema.object({ version: schema.number() }).strip() })
    .strip();

/** Exercise hardware signing and root rotation through the real TUF client on loopback. */
export async function rehearseKey(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // select the rehearsal hardware and generate disposable keys for every other role
    const { crypto, files } = environment;
    const hardware = await readHardware(arguments_, environment);
    const [first, second, third] = [
        await SigningKey.generate("ed25519", crypto),
        await SigningKey.generate("ed25519", crypto),
        await SigningKey.generate("ed25519", crypto),
    ];
    const keys = {
        targets: await SigningKey.generate("ed25519", crypto),
        snapshot: await SigningKey.generate("ed25519", crypto),
        timestamp: await SigningKey.generate("ed25519", crypto),
    };
    const expires = new Date(environment.clock.now() + ROOT_LIFETIME_MILLISECONDS).toISOString();

    // serve the temporary repository on loopback
    const directory = await files.makeTemporaryDirectory("destack-signing-rehearsal-");
    const repository = join(directory, "repository");
    const server = serveRepository(repository, environment);
    try {
        // create an initial two-of-three root using one real hardware signature
        const roots = [hardware.public, first.public, second.public];
        const root = await signRoot(1, roots, keys, expires, hardware, [first]);
        TrustedRoot.verify(root);
        const content = Buffer.from("Destack signing rehearsal\n");
        const distribution = await writeArtifact(directory, content, environment);
        await SignedRepository.create(repository, 1, root, keys, distribution, environment);

        // authenticate metadata and download a digest-verified target through tuf-js
        const options = await createClientOptions(directory, server.url, root, environment);
        await verifyTarget(options, content, environment);

        // replace a root key and require authorization from both old and new key sets
        const rotatedRoots = [hardware.public, second.public, third.public];
        const rotated = await signRoot(2, rotatedRoots, keys, expires, hardware, [first, second]);
        TrustedRoot.verify(rotated, root);
        await SignedRepository.create(repository, 2, rotated, keys, distribution, environment);
        await verifyRotation(options, environment);
        print(
            "hardware RSA-PSS signing, target verification and root rotation passed",
            environment,
        );
    } finally {
        await server.stop(true);
        await files.remove(directory, { recursive: true, force: true });
    }
}

/** Select the rehearsal hardware without loading any production credential. */
async function readHardware(
    arguments_: readonly string[],
    environment: Environment<HostCapability>,
): Promise<HardwareKey> {
    // require the public key, serial and PKCS#11 module arguments
    const [publicPath, serial, module, ...extra] = arguments_;
    if (
        publicPath === undefined ||
        serial === undefined ||
        module === undefined ||
        extra.length > 0
    ) {
        throw new Error("usage: key rehearse <public-key.pem> <serial> <pkcs11-module>");
    }

    // read the public key the hardware signs with
    const key = readPublicKey(await environment.files.readTextFile(publicPath));

    return new HardwareKey(key, serial, module, environment);
}

/** Serve the metadata and target files of a repository directory on a loopback port. */
function serveRepository(
    repository: string,
    environment: Environment<"files" | "server">,
): Listener {
    return environment.server.http<undefined>({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            // expose only the temporary repository's public files
            const path = new URL(request.url).pathname;
            if (!/^\/(metadata|targets)\/[^/]+$/u.test(path)) {
                return new Response(null, { status: 404 });
            }
            const bytes = await environment.files
                .readFile(join(repository, path))
                .catch((error: unknown) => FileSystemError.absent(error));

            return bytes === undefined ? new Response(null, { status: 404 }) : new Response(bytes);
        },
    });
}

/** Create a root signed by the hardware key and the disposable signers. */
async function signRoot(
    version: number,
    roots: Key[],
    keys: ReleaseKeys,
    expires: string,
    hardware: HardwareKey,
    signers: SigningKey[],
): Promise<Metadata<Root>> {
    // create the root over the online role keys
    const publicKeys = {
        targets: keys.targets.public,
        snapshot: keys.snapshot.public,
        timestamp: keys.timestamp.public,
    };
    const root = TrustedRoot.create(version, roots, publicKeys, expires, hardware.environment);

    // sign with the YubiKey and check its signature before the disposable signatures
    print(`sign rehearsal root ${version} with the selected YubiKey`, hardware.environment);
    await hardware.sign(root);
    hardware.public.verifySignature(root);
    for (const signer of signers) {
        await signer.sign(root);
    }

    return root;
}

/** Write the rehearsal artifact and describe it as the one distribution to publish. */
async function writeArtifact(
    directory: string,
    content: Buffer,
    environment: Environment<"files" | "process">,
): Promise<Distribution[]> {
    // write the content as the archive of an arm64 macOS release
    const archive = join(directory, "artifact");
    await environment.files.writeFile(archive, content);
    const commit = await readCommit(import.meta.dirname, environment);

    return [
        {
            kind: "installer",
            target: "aarch64-apple-darwin",
            version: "2026.9.1",
            commit,
            archive,
            format: "tar.gz",
        },
    ];
}

/** Trust a root in a fresh client metadata directory, answering the server's client options. */
async function createClientOptions(
    directory: string,
    url: URL,
    root: Metadata<Root>,
    environment: Environment<"files">,
): Promise<UpdaterOptions> {
    // write the root as the client's trusted root
    const { files } = environment;
    const metadata = join(directory, "client");
    await files.mkdir(metadata);
    await files.writeFile(join(metadata, "root.json"), SignedRepository.encode(root));

    // download targets into their own directory
    const download = join(directory, "download");
    const options = {
        metadataDir: metadata,
        metadataBaseUrl: new URL("/metadata/", url).href,
        targetDir: download,
        targetBaseUrl: new URL("/targets/", url).href,
    };
    await files.mkdir(download);

    return options;
}

/** Refresh the client and require the downloaded target to equal the signed content. */
async function verifyTarget(
    options: UpdaterOptions,
    content: Buffer,
    environment: Environment<"files">,
): Promise<void> {
    // download the target through the authenticated metadata
    const client = new Updater(options);
    await client.refresh();
    const target = await client.getTargetInfo("aarch64-apple-darwin.tar.gz");
    const downloaded =
        target === undefined
            ? undefined
            : await environment.files.readFile(await client.downloadTarget(target));
    if (downloaded === undefined || !content.equals(downloaded)) {
        throw new Error("rehearsal target differs from its signed content");
    }
}

/** Refresh a new client and require it to trust root 2. */
async function verifyRotation(
    options: UpdaterOptions,
    environment: Environment<"files">,
): Promise<void> {
    // read the trusted root the client persisted
    const updated = new Updater(options);
    await updated.refresh();
    const text = await environment.files.readTextFile(join(options.metadataDir, "root.json"));
    const trusted = TrustedVersion.parse(JSON.parse(text));
    if (trusted.signed.version !== 2) {
        throw new Error("client did not accept the root rotation");
    }
}
