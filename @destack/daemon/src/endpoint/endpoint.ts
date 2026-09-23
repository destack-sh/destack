import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { schema } from "@destack/schema";
import { DaemonError } from "../error/index.ts";

/** Authenticated loopback endpoint published by one daemon process. */
export const Endpoint = schema.object({
    /** Ephemeral IPv4 loopback port. */
    port: schema.number().int().min(1).max(65535),
    /** Private owner credential, replaced after each restart. */
    token: schema.string().regex(/^[0-9a-f]{64}$/),
});

/** Authenticated loopback endpoint published by one daemon process. */
export type Endpoint = schema.Infer<typeof Endpoint>;

/** Read the current endpoint without creating or repairing daemon state. */
export async function readEndpoint(directory: string): Promise<Endpoint> {
    let text: string;
    try {
        text = await readFile(join(directory, "daemon.json"), "utf8");
    } catch (cause) {
        if ((cause as NodeJS.ErrnoException).code === "ENOENT") {
            throw new DaemonError("NOT_RUNNING", "the Destack daemon is not running", { cause });
        }
        throw cause;
    }

    // report corrupt discovery separately from a stopped daemon
    try {
        return Endpoint.parse(JSON.parse(text));
    } catch (cause) {
        throw new DaemonError("INVALID_ENDPOINT", "invalid local daemon endpoint", { cause });
    }
}
