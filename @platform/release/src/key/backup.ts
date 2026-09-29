import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";

/** An age identity retained in Bitwarden and used to verify the recovery archive. */
export class BackupKey {
    /** Private age identity text, never written to the backup medium. */
    readonly identity: string;
    /** Public age recipient used for encryption. */
    readonly recipient: string;

    /** Read an existing recovery key or generate one using the installed age implementation. */
    constructor(identity?: string) {
        this.identity = identity ?? run("age-keygen", [], undefined).toString();
        this.recipient = run("age-keygen", ["-y"], this.identity).toString().trim();
    }

    /** Encrypt an archive and verify every decrypted byte after reading it from disk. */
    async write(archive: Buffer, output: string): Promise<void> {
        // refuse to overwrite an existing backup, including an incomplete one
        const encrypted = run("age", ["-r", this.recipient], archive);
        await writeFile(output, encrypted, { flag: "wx", mode: 0o600, flush: true });

        // read the actual destination and authenticate its complete encrypted payload
        const stored = await readFile(output);
        if (!stored.equals(encrypted)) {
            throw new Error("encrypted backup readback differs from the written archive");
        }
        const restored = run("age", ["-d", "-i", "-", output], this.identity);
        if (!restored.equals(archive)) {
            throw new Error("decrypted backup differs from the original archive");
        }
    }
}

/** Read the small signing directory into memory without writing a plaintext archive. */
export function readBackup(directory: string): Buffer {
    return run("tar", ["-czf", "-", "-C", directory, "."], undefined);
}

/** Run standard tools through private pipes without exposing vault sessions or output. */
function run(command: string, arguments_: string[], input: string | Buffer | undefined): Buffer {
    // keep the Bitwarden session out of unrelated child processes
    const environment = { ...process.env };
    delete environment.BW_SESSION;
    delete environment.BW_PASSWORD;
    const result = spawnSync(command, arguments_, {
        input,
        env: environment,
        stdio: ["pipe", "pipe", "pipe"],
        timeout: 30_000,
        maxBuffer: 32 * 1024 * 1024,
    });
    if (result.error || result.status !== 0) {
        throw new Error(
            `${command} failed while preparing the recovery backup; private output was withheld`,
        );
    }

    return result.stdout;
}
