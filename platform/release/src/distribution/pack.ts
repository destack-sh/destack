import { writeFile } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { version } from "./index.ts";
import { Release } from "@destack/update/release";

/** Repository containing the compiled target directories. */
const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

/** Package one compiled target and record its digest. */
async function pack(): Promise<void> {
    // package the desktop and standalone CLI together
    const target = new Release(version, process.argv[2] ?? Release.target()).target;
    const directory = join(ROOT, "dist", version);
    const name = `destack-${version}-${target}.tar.gz`;
    const child = Bun.spawn(
        [
            "tar",
            "-czf",
            join(directory, name),
            "-C",
            join(directory, target),
            target.includes("apple") ? "Destack.app" : "Destack",
            "bin",
        ],
        {
            env: { ...process.env, COPYFILE_DISABLE: "1" },
            stdout: "inherit",
            stderr: "inherit",
        },
    );
    const code = await child.exited;
    if (code !== 0) {
        throw new Error(`archive creation failed: ${code}`);
    }

    // record the exact bytes published for this target
    const hash = createHash("sha256");
    let size = 0;
    for await (const bytes of createReadStream(join(directory, name))) {
        hash.update(bytes);
        size += bytes.length;
    }
    const sha256 = hash.digest("hex");

    // persist the digest and size from the same bounded read
    await writeFile(
        join(directory, `${target}.json`),
        JSON.stringify(
            {
                target,
                file: name,
                size,
                sha256,
            },
            null,
            4,
        ) + "\n",
    );
    console.log(`${name}: ${size} bytes, SHA-256 ${sha256}`);
}

await pack();
