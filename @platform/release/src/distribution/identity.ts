import type { Environment } from "@destack/environment";
import { Stage } from "@destack/update/release";

/** Read the stage a build selects from its environment, local development by default. */
export function selectedChannel(host: Environment<"process">): Stage {
    return Stage.of(host.process.variables["DESTACK_STAGE"] ?? "local");
}
