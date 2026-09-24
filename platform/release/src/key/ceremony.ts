import { readFile, writeFile } from "node:fs/promises";
import { join, isAbsolute } from "node:path";
import { Key } from "@tufjs/models";
import { readPublicKey } from "./key.ts";
import { HardwareKey } from "./hardware.ts";
import { createRoot, readRoot, verifyRoot } from "./root.ts";
import { encode } from "../repository/repository.ts";

/** Prepare, sign or verify a root ceremony document without overwriting its input. */
async function main(): Promise<void> {
    const [command, ...arguments_] = process.argv.slice(2);

    // prepare initial trust or the next consecutive rotation from public keys
    if (command === "prepare") {
        const [directory, first, second, third, expires, output, previousPath, ...extra] =
            arguments_;
        if (!directory || !first || !second || !third || !expires || !output || extra.length) {
            throw new Error(
                "usage: root.ts prepare <online-public-directory> <A.pem> <B.pem> <C.pem> <expires> <output> [previous-root]",
            );
        }
        const previous = previousPath ? await readRoot(previousPath) : undefined;
        if (previous) {
            previous.verifyDelegate("root", previous);
        }
        const roots = await Promise.all(
            [first, second, third].map(async (path) => readPublicKey(await readFile(path, "utf8"))),
        );
        const keys = await Promise.all(
            ["targets", "snapshot", "timestamp"].map(async (role) => {
                const document = JSON.parse(
                    await readFile(join(directory, `${role}.json`), "utf8"),
                );

                return Key.fromJSON(document.keyid, document);
            }),
        );
        const root = createRoot(
            previous ? previous.signed.version + 1 : 1,
            roots,
            {
                targets: keys[0]!,
                snapshot: keys[1]!,
                timestamp: keys[2]!,
            },
            expires,
        );
        await writeFile(output, encode(root), { flag: "wx", mode: 0o644 });
    }
    // append one hardware signature and verify it before writing the new document
    else if (command === "sign") {
        const [input, publicPath, serial, module, output, previousPath, ...extra] = arguments_;
        if (
            !input ||
            !publicPath ||
            !serial ||
            !module ||
            !isAbsolute(module) ||
            !output ||
            extra.length
        ) {
            throw new Error(
                "usage: root.ts sign <input> <public-key.pem> <serial> <absolute-pkcs11-module> <output> [previous-root]",
            );
        }
        const root = await readRoot(input);
        const previous = previousPath ? await readRoot(previousPath) : undefined;
        if (previous) {
            previous.verifyDelegate("root", previous);
            if (root.signed.version !== previous.signed.version + 1) {
                throw new Error("root rotation must advance by one version");
            }
        }
        const key = readPublicKey(await readFile(publicPath, "utf8"));
        const authorized = [root, ...(previous ? [previous] : [])].some((metadata) =>
            metadata.signed.roles.root?.keyIDs.includes(key.keyID),
        );
        if (!authorized) {
            throw new Error("signing key is not authorized by either root");
        }
        const hardware = new HardwareKey(key, serial, module);
        console.log(JSON.stringify(root.signed.toJSON(), null, 4));
        root.sign((bytes) => hardware.sign(bytes));
        key.verifySignature(root);
        await writeFile(output, encode(root), { flag: "wx", mode: 0o644 });
    }
    // check both quorums before a root becomes eligible for publication
    else if (command === "verify") {
        const [input, previousPath, ...extra] = arguments_;
        if (!input || extra.length) {
            throw new Error("usage: root.ts verify <root> [previous-root]");
        }
        const root = await readRoot(input);
        verifyRoot(root, previousPath ? await readRoot(previousPath) : undefined);
        console.log(`verified root version ${root.signed.version}`);
    } else {
        throw new Error("usage: root.ts <prepare|sign|verify> ...");
    }
}

await main();
