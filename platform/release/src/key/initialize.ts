import { mkdir, writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { SigningKey } from "./key.ts";

/** Generate independent online keys in a new private directory without creating root keys. */
export async function initialize(directory: string): Promise<void> {
    if (!isAbsolute(directory)) {
        throw new Error("use an absolute private key directory");
    }

    // refuse to overwrite an existing key set
    await mkdir(directory, { mode: 0o700 });
    for (const role of ["targets", "snapshot", "timestamp"]) {
        const key = SigningKey.generate();
        await writeFile(join(directory, `${role}.pem`), key.private, { flag: "wx", mode: 0o600 });
        await writeFile(
            join(directory, `${role}.json`),
            JSON.stringify(
                {
                    keyid: key.public.keyID,
                    ...key.public.toJSON(),
                },
                null,
                4,
            ) + "\n",
            { flag: "wx", mode: 0o644 },
        );
    }
}
