import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";

/** Read the real host, for the integration tests that need the operating system itself. */
export function testHost(): Environment<"files" | "process" | "server" | "platform" | "workers"> {
    return BunEnvironment.system;
}
