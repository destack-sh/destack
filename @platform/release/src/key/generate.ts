import type { Environment, HostCapability } from "@destack/environment";
import { initialize } from "./initialize.ts";

/** Generate independent online keys into a new private directory. */
export async function generateKeys(
    environment: Environment<HostCapability>,
    arguments_: readonly string[],
): Promise<void> {
    // require exactly the new private directory
    const [directory, ...extra] = arguments_;
    if (directory === undefined || extra.length > 0) {
        throw new Error("usage: key generate <new-private-directory>");
    }
    await initialize(directory, environment);
}
