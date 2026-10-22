import { Failure } from "@destack/error";
import { HandlerAnswer, HookServer } from "@destack/object";
import { present } from "@destack/schema";
import type { PackageHandle } from "@destack/object";

import type { WorkloadContext, WorkloadImplementation } from "@destack/object/workload";
import stackPackage, { HOOK_PATH } from "../package.ts";

/** The name of the workload a build derives to serve its package's object hooks. */
export const HOOK_WORKLOAD = "hooks";

/** Start serving the hooks of the types packages route, reaching the store through the host. */
export function serveHooks(
    packages: readonly PackageHandle[],
): (context: WorkloadContext) => WorkloadImplementation {
    return (context) => {
        // reach the space's stack as the installation
        const server = present(context.server, "the server context of the installation with hooks");
        const stack = server.at(stackPackage.name);

        // answer the store's hook requests, sending theirs in each mutation's session
        const hooks = new HookServer({
            packages,
            server,
            session: async (asked) => {
                const answer = await stack.fetch(
                    new Request(`${stack.url}${HOOK_PATH}`, {
                        method: "POST",
                        headers: { "content-type": "application/json" },
                        body: JSON.stringify(asked),
                    }),
                );
                if (!answer.ok) {
                    throw Failure.error(await Failure.read(answer));
                }

                return HandlerAnswer.parse(await answer.json());
            },
        });

        return { packages: [], hooks };
    };
}
