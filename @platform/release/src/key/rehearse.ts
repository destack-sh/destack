import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Updater } from "tuf-js";
import { HardwareKey } from "./hardware.ts";
import { readPublicKey, SigningKey } from "./key.ts";
import { createRoot, verifyRoot } from "./root.ts";
import { createRepository, encode } from "../repository/repository.ts";

/** Exercise hardware signing and root rotation through the real TUF client on loopback. */
async function rehearse(): Promise<void> {
    // select the rehearsal hardware without loading any production credential
    const [publicPath, serial, module, ...extra] = process.argv.slice(2);
    if (!publicPath || !serial || !module || extra.length) {
        throw new Error("usage: rehearse.ts <public-key.pem> <serial> <pkcs11-module>");
    }
    const hardware = new HardwareKey(
        readPublicKey(await readFile(publicPath, "utf8")),
        serial,
        module,
    );
    const roots = [SigningKey.generate(), SigningKey.generate(), SigningKey.generate()];
    const keys = {
        targets: SigningKey.generate(),
        snapshot: SigningKey.generate(),
        timestamp: SigningKey.generate(),
    };
    const publicKeys = {
        targets: keys.targets.public,
        snapshot: keys.snapshot.public,
        timestamp: keys.timestamp.public,
    };
    const expires = new Date(Date.now() + 365 * 86_400_000).toISOString();
    const directory = await mkdtemp(join(tmpdir(), "destack-signing-rehearsal-"));
    const repository = join(directory, "repository");
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            // expose only the temporary repository's public files
            const path = new URL(request.url).pathname;
            if (!/^\/(metadata|targets)\/[^/]+$/.test(path)) {
                return new Response(null, { status: 404 });
            }
            const file = Bun.file(join(repository, path));

            return (await file.exists()) ? new Response(file) : new Response(null, { status: 404 });
        },
    });
    try {
        // create an initial two-of-three root using one real hardware signature
        const root = createRoot(
            1,
            [hardware.public, roots[0]!.public, roots[1]!.public],
            publicKeys,
            expires,
        );
        console.log("sign rehearsal root 1 with the selected YubiKey");
        root.sign((bytes) => hardware.sign(bytes));
        hardware.public.verifySignature(root);
        root.sign((bytes) => roots[0]!.sign(bytes));
        verifyRoot(root);
        const archive = join(directory, "artifact");
        const content = Buffer.from("Destack signing rehearsal\n");
        await writeFile(archive, content);
        const distribution = [{ target: "aarch64-apple-darwin", version: "2026.9.1", archive }];
        await createRepository(repository, 1, root, keys, distribution);

        // authenticate metadata and download a digest-verified target through tuf-js
        const metadata = join(directory, "client");
        await mkdir(metadata);
        await writeFile(join(metadata, "root.json"), encode(root));
        const options = {
            metadataDir: metadata,
            metadataBaseUrl: new URL("/metadata/", server.url).href,
            targetDir: join(directory, "download"),
            targetBaseUrl: new URL("/targets/", server.url).href,
        };
        await mkdir(options.targetDir);
        const client = new Updater(options);
        await client.refresh();
        const target = await client.getTargetInfo("aarch64-apple-darwin.tar.gz");
        if (!target || !(await readFile(await client.downloadTarget(target))).equals(content)) {
            throw new Error("rehearsal target differs from its signed content");
        }

        // replace a root key and require authorization from both old and new key sets
        const rotated = createRoot(
            2,
            [hardware.public, roots[1]!.public, roots[2]!.public],
            publicKeys,
            expires,
        );
        console.log("sign rehearsal root 2 with the selected YubiKey");
        rotated.sign((bytes) => hardware.sign(bytes));
        hardware.public.verifySignature(rotated);
        for (const key of roots.slice(0, 2)) {
            rotated.sign((bytes) => key.sign(bytes));
        }
        verifyRoot(rotated, root);
        await createRepository(repository, 2, rotated, keys, distribution);
        const updated = new Updater(options);
        await updated.refresh();
        const trusted = JSON.parse(await readFile(join(metadata, "root.json"), "utf8"));
        if (trusted.signed.version !== 2) {
            throw new Error("client did not accept the root rotation");
        }
        console.log("hardware RSA-PSS signing, target verification and root rotation passed");
    } finally {
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
}

await rehearse();
