import { readFile, writeFile } from "node:fs/promises";
import { join, isAbsolute } from "node:path";
import { Key, type Metadata, type Root } from "@tufjs/models";
import { HardwareKey } from "../yubico/hardware.ts";
import { readPublicKey, SignedRepository, TrustedRoot } from "@destack/update/publish";
import { schema } from "@destack/schema";
import { print } from "../output/index.ts";

/** A public online key document as initialization writes it. */
const OnlineKey = schema.object({
    keyid: schema.string(),
    keytype: schema.string(),
    scheme: schema.string(),
    keyval: schema.record(schema.string(), schema.string()),
});

/** Prepare, sign or verify a root ceremony document without overwriting its input. */
async function main(): Promise<void> {
    const [command, ...arguments_] = process.argv.slice(2);

    // prepare initial trust or the next consecutive rotation from public keys
    if (command === "prepare") {
        await prepare(arguments_);
    }
    // append one hardware signature and verify it before writing the new document
    else if (command === "sign") {
        await sign(arguments_);
    }
    // check both quorums before a root becomes eligible for publication
    else if (command === "verify") {
        await verify(arguments_);
    }
    // refuse every other command
    else {
        throw new Error("usage: ceremony.ts <prepare|sign|verify> ...");
    }
}

/** Write an unsigned root naming three hardware keys and the online role keys. */
async function prepare(arguments_: string[]): Promise<void> {
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
    const previous = await readPrevious(previousPath);
    previous?.verifyDelegate("root", previous);

    // read the hardware root keys and the online role keys
    const roots = await Promise.all(
        [first, second, third].map(async (path) => readPublicKey(await readFile(path, "utf8"))),
    );
    const [targets, snapshot, timestamp] = await Promise.all([
        readOnlineKey(directory, "targets"),
        readOnlineKey(directory, "snapshot"),
        readOnlineKey(directory, "timestamp"),
    ]);

    // write the unsigned root without replacing an existing document
    const version = previous === undefined ? 1 : previous.signed.version + 1;
    const root = TrustedRoot.create(version, roots, { targets, snapshot, timestamp }, expires);
    await writeFile(output, SignedRepository.encode(root), { flag: "wx", mode: 0o644 });
}

/** Append one hardware signature to a root and verify it before writing the new document. */
async function sign(arguments_: string[]): Promise<void> {
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
    const root = await TrustedRoot.read(input);
    const previous = await readPrevious(previousPath);
    if (previous !== undefined) {
        previous.verifyDelegate("root", previous);
        if (root.signed.version !== previous.signed.version + 1) {
            throw new Error("root rotation must advance by one version");
        }
    }

    // require the signing key in the new root or the previous one
    const key = readPublicKey(await readFile(publicPath, "utf8"));
    const authorized = [root, previous].some(
        (metadata) => metadata?.signed.roles["root"]?.keyIDs.includes(key.keyID) === true,
    );
    if (!authorized) {
        throw new Error("signing key is not authorized by either root");
    }

    // show the signed content, sign it on the hardware and verify the signature
    const hardware = new HardwareKey(key, serial, module);
    print(JSON.stringify(root.signed.toJSON(), null, 4));
    root.sign((bytes) => hardware.sign(bytes));
    key.verifySignature(root);
    await writeFile(output, SignedRepository.encode(root), { flag: "wx", mode: 0o644 });
}

/** Verify a root's quorum and, for a rotation, the previous root's quorum. */
async function verify(arguments_: string[]): Promise<void> {
    // verify the root against the previous one when rotating
    const [input, previousPath, ...extra] = arguments_;
    if (input === undefined || extra.length > 0) {
        throw new Error("usage: ceremony.ts verify <root> [previous-root]");
    }
    const root = await TrustedRoot.read(input);
    TrustedRoot.verify(root, await readPrevious(previousPath));
    print(`verified root version ${root.signed.version}`);
}

/** Read the previous root of a rotation, absent for initial trust. */
async function readPrevious(path: string | undefined): Promise<Metadata<Root> | undefined> {
    return path === undefined ? undefined : TrustedRoot.read(path);
}

/** Read one online role's public key document from the online public directory. */
async function readOnlineKey(directory: string, role: string): Promise<Key> {
    const document = OnlineKey.parse(
        JSON.parse(await readFile(join(directory, `${role}.json`), "utf8")),
    );

    return Key.fromJSON(document.keyid, document);
}

await main();
