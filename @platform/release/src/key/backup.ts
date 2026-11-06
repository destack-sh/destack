import type { Environment } from "@destack/environment";
import { communicate } from "../distribution/command.ts";

/** How long one backup tool may run, in milliseconds. */
const TOOL_TIMEOUT_MILLISECONDS = 30_000;

/** The variables of the unlocked vault, which no backup tool receives. */
const VAULT_VARIABLES: ReadonlySet<string> = new Set(["BW_SESSION", "BW_PASSWORD"]);

/** An age identity retained in Bitwarden and used to verify the recovery archive. */
export class BackupKey {
    /** Private age identity text, never written to the backup medium. */
    readonly identity: string;
    /** Public age recipient used for encryption. */
    readonly recipient: string;
    /** The host the backup tools run on. */
    readonly environment: Environment<"files" | "process">;

    /** Keep an identity and its recipient. */
    private constructor(
        identity: string,
        recipient: string,
        environment: Environment<"files" | "process">,
    ) {
        this.identity = identity;
        this.recipient = recipient;
        this.environment = environment;
    }

    /** Generate a recovery key with the installed age implementation. */
    static async generate(environment: Environment<"files" | "process">): Promise<BackupKey> {
        const identity = await run("age-keygen", [], undefined, environment);

        return await BackupKey.of(new TextDecoder().decode(identity), environment);
    }

    /** Read an existing recovery key, deriving its recipient with the installed age. */
    static async of(
        identity: string,
        environment: Environment<"files" | "process">,
    ): Promise<BackupKey> {
        const recipient = await run("age-keygen", ["-y"], identity, environment);

        return new BackupKey(identity, new TextDecoder().decode(recipient).trim(), environment);
    }

    /** Encrypt an archive and verify every decrypted byte after reading it from disk. */
    async write(archive: Uint8Array, output: string): Promise<void> {
        // refuse to overwrite an existing backup, including an incomplete one
        const { environment } = this;
        const encrypted = await run("age", ["-r", this.recipient], archive, environment);
        {
            await using file = await environment.files.open(output, {
                write: true,
                createNew: true,
                mode: 0o600,
            });
            const writer = file.writable().getWriter();
            await writer.write(encrypted);
            await writer.close();
            await file.sync();
        }

        // read the actual destination and authenticate its complete encrypted payload
        const stored = await environment.files.readFile(output);
        if (!Buffer.from(stored).equals(encrypted)) {
            throw new Error("encrypted backup readback differs from the written archive");
        }
        const restored = await run("age", ["-d", "-i", "-", output], this.identity, environment);
        if (!Buffer.from(restored).equals(archive)) {
            throw new Error("decrypted backup differs from the original archive");
        }
    }
}

/** Read the small signing directory into memory without writing a plaintext archive. */
export async function readBackup(
    directory: string,
    environment: Environment<"process">,
): Promise<Uint8Array> {
    return await run("tar", ["-czf", "-", "-C", directory, "."], undefined, environment);
}

/** Run standard tools through private pipes without exposing vault sessions or output. */
async function run(
    command: string,
    arguments_: string[],
    input: string | Uint8Array | undefined,
    environment: Environment<"process">,
): Promise<Uint8Array> {
    // keep the Bitwarden session out of unrelated child processes
    const variables = Object.fromEntries(
        Object.entries(environment.process.variables).filter(
            ([name]) => !VAULT_VARIABLES.has(name),
        ),
    );
    const result = await communicate(command, arguments_, environment, {
        ...(input === undefined ? {} : { input }),
        variables,
        timeout: TOOL_TIMEOUT_MILLISECONDS,
    });

    // refuse a failed tool without its private output
    if (result.code !== 0) {
        throw new Error(
            `${command} failed while preparing the recovery backup; private output was withheld`,
        );
    }

    return result.stdout;
}
