import { isAbsolute, join } from "node:path";
import type { Environment } from "@destack/environment";
import { SigningKey } from "@destack/update/publish";

/** Generate independent online keys in a new private directory without creating root keys. */
export async function initialize(
    directory: string,
    environment: Environment<"files">,
): Promise<void> {
    if (!isAbsolute(directory)) {
        throw new Error("use an absolute private key directory");
    }

    // refuse to overwrite an existing key set
    const { files } = environment;
    await files.mkdir(directory, { mode: 0o700 });
    for (const role of ["targets", "snapshot", "timestamp"]) {
        const key = await SigningKey.generate("ed25519", environment.crypto);
        await files.writeTextFile(join(directory, `${role}.pem`), key.private, {
            createNew: true,
            mode: 0o600,
        });
        await files.writeTextFile(
            join(directory, `${role}.json`),
            JSON.stringify(
                {
                    keyid: key.public.keyID,
                    ...key.public.toJSON(),
                },
                null,
                4,
            ) + "\n",
            { createNew: true, mode: 0o644 },
        );
    }
}
