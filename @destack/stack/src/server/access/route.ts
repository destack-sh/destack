import { hostname } from "@destack/account/object";
import { type MethodContext } from "@destack/object";
import { route } from "../../object/index.ts";
import type {} from "@destack/package/import-meta";
import { StackError } from "../../error/index.ts";

/** Implement the routes of a space, refused on a machine the account's hostnames do not reach. */
export function implementRoute(isServingHostnames: boolean) {
    return route.implement({
        methods: {
            create: async (call, next) => {
                // refuse a route no request would reach, then a hostname outside the account
                if (!isServingHostnames) {
                    throw StackError.of(
                        "HOSTNAME_NOT_SERVED",
                        `requests for ${call.input.hostname} do not reach this space's machine`,
                    );
                }
                await requireHostname(call, call.input.hostname);

                return next();
            },
        },
    });
}

/** Require a hostname of the space's account by name, the only hostnames its store follows. */
async function requireHostname(call: MethodContext, name: string): Promise<void> {
    // refuse a hostname the space's account lacks
    const [owned] = await call.read(hostname).findMany({ where: { name } });
    if (owned === undefined) {
        throw StackError.of("HOSTNAME_NOT_FOUND", `hostname ${name} not found`);
    }
}
