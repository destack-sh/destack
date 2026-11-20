import type { Environment, HostCapability } from "@destack/environment";
import { join, isAbsolute } from "node:path";
import { Key, type Metadata, type Root } from "@tufjs/models";
import { HardwareKey } from "./hardware.ts";
import { readPublicKey, SignedRepository, TrustedRoot } from "@destack/update/publish";
import { schema } from "@destack/schema";
import { print } from "../distribution/output.ts";

/** A public online key document as initialization writes it. */
const OnlineKey = schema.object({
    keyid: schema.string(),
    keytype: schema.string(),
    scheme: schema.string(),
    keyval: schema.record(schema.string(), schema.string()),
});

/** Prepare, sign or verify a root ceremony document without overwriting its input. */
export async function runCeremony(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    const [command, ...rest] = arguments_;

    // prepare initial trust or the next consecutive rotation from public keys
    if (command === "prepare") {
        await prepare(rest, environment);
    }
    // append one hardware signature and verify it before writing the new document
    else if (command === "sign") {
        await sign(rest, environment);
    }
    // check both quorums before a root becomes eligible for publication
    else if (command === "verify") {
        await verify(rest, environment);
    }
    // refuse every other command
    else {
        throw new Error("usage: key ceremony <prepare|sign|verify> ...");
    }
}

/** Write an unsigned root with three hardware keys and the online role keys. */
async function prepare(
    arguments_: readonly string[],
    environment: Environment<HostCapability>,
): Promise<void> {
    // require every positional argument except the optional previous root
    const [directory, first, second, third, expires, output, previousPath, ...extra] = arguments_;
    if (
        directory === undefined ||
        first === undefined ||
        second === undefined ||
        third === undefined ||
        expires === undefined ||
        output === undefined ||
        extra.length > 0
    ) {
        throw new Error(
            "usage: ceremony.ts prepare <online-public-directory> <A.pem> <B.pem> <C.pem> <expires> <output> [previous-root]",
        );
    }

    // authenticate the previous root before numbering its successor
    const previous = await readPrevious(previousPath, environment);
    previous?.verifyDelegate("root", previous);

    // read the hardware root keys and the online role keys
    const { files } = environment;
    const roots = await Promise.all(
        [first, second, third].map(async (path) => readPublicKey(await files.readTextFile(path))),
    );
    const [targets, snapshot, timestamp] = await Promise.all([
        readOnlineKey(directory, "targets", environment),
        readOnlineKey(directory, "snapshot", environment),
        readOnlineKey(directory, "timestamp", environment),
    ]);

    // write the unsigned root without replacing an existing document
    const version = previous === undefined ? 1 : previous.signed.version + 1;
    const root = TrustedRoot.create(
        version,
        roots,
        { targets, snapshot, timestamp },
        expires,
        environment,
    );
    await files.writeFile(output, SignedRepository.encode(root), {
        createNew: true,
        mode: 0o644,
    });
}

/** Append one hardware signature to a root and verify it before writing the new document. */
async function sign(
    arguments_: readonly string[],
    environment: Environment<HostCapability>,
): Promise<void> {
    // require the root, the hardware key and an absolute PKCS#11 module
    const [input, publicPath, serial, module, output, previousPath, ...extra] = arguments_;
    if (
        input === undefined ||
        publicPath === undefined ||
        serial === undefined ||
        module === undefined ||
        !isAbsolute(module) ||
        output === undefined ||
        extra.length > 0
    ) {
        throw new Error(
            "usage: ceremony.ts sign <input> <public-key.pem> <serial> <absolute-pkcs11-module> <output> [previous-root]",
        );
    }

    // require a rotation to advance the authenticated previous root by one version
    const root = await TrustedRoot.read(input, environment);
    const previous = await readPrevious(previousPath, environment);
    if (previous !== undefined) {
        previous.verifyDelegate("root", previous);
        if (root.signed.version !== previous.signed.version + 1) {
            throw new Error("root rotation must advance by one version");
        }
    }

    // require the signing key in the new root or the previous one
    const { files } = environment;
    const key = readPublicKey(await files.readTextFile(publicPath));
    const authorized = [root, previous].some(
        (metadata) => metadata?.signed.roles["root"]?.keyIDs.includes(key.keyID) === true,
    );
    if (!authorized) {
        throw new Error("signing key is not authorized by either root");
    }

    // show the signed content, sign it on the hardware and verify the signature
    const hardware = new HardwareKey(key, serial, module, environment);
    print(JSON.stringify(root.signed.toJSON(), null, 4), environment);
    await hardware.sign(root);
    key.verifySignature(root);
    await files.writeFile(output, SignedRepository.encode(root), {
        createNew: true,
        mode: 0o644,
    });
}

/** Verify a root's quorum and, for a rotation, the previous root's quorum. */
async function verify(
    arguments_: readonly string[],
    environment: Environment<HostCapability>,
): Promise<void> {
    // verify the root against the previous one when rotating
    const [input, previousPath, ...extra] = arguments_;
    if (input === undefined || extra.length > 0) {
        throw new Error("usage: ceremony.ts verify <root> [previous-root]");
    }
    const root = await TrustedRoot.read(input, environment);
    TrustedRoot.verify(root, await readPrevious(previousPath, environment));
    print(`verified root version ${root.signed.version}`, environment);
}

/** Read the previous root of a rotation, absent for initial trust. */
async function readPrevious(
    path: string | undefined,
    environment: Environment<HostCapability>,
): Promise<Metadata<Root> | undefined> {
    return path === undefined ? undefined : TrustedRoot.read(path, environment);
}

/** Read one online role's public key document from the online public directory. */
async function readOnlineKey(
    directory: string,
    role: string,
    environment: Environment<"files">,
): Promise<Key> {
    const text = await environment.files.readTextFile(join(directory, `${role}.json`));
    const document = OnlineKey.parse(JSON.parse(text));

    return Key.fromJSON(document.keyid, document);
}
