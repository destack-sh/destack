import { hostname, space } from "@destack/account/object";
import { type Transaction } from "@destack/db";
import { type Identifier, present, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { Installation, installation, route } from "../../object/index.ts";

/** Serve the routes a stack declares in its space. */
export function serveRoutes() {
    return declaredRoute;
}

/** Routes a stack declares in its space. */
const declaredRoute = route.implement({
    declaration: {
        after: [installation],
        resolve: async (_name, declared, stack) => {
            // require a hostname of the space's account
            const served = await requireHostname(
                stack.transaction,
                schema.identifier("space").parse(stack.scope),
                declared.hostname,
            );

            // redirect
            const destination = declared.destination;
            const matched = { hostname: served, path: declared.path, match: declared.match };
            if ("redirect" in destination) {
                return {
                    ...matched,
                    installationId: null,
                    view: null,
                    adapter: null,
                    redirect: destination.redirect,
                    redirectStatus: destination.status,
                };
            }

            // serve a view or an adapter of an installation of the space
            return {
                ...matched,
                installationId: await Installation.resolve(destination.installation, stack),
                view: "view" in destination ? destination.view : null,
                adapter: "adapter" in destination ? destination.adapter : null,
                redirect: null,
                redirectStatus: null,
            };
        },
        values: (_name, resolved) => resolved,
    },
});

/** Find a hostname of the space's account by name. */
async function requireHostname(
    transaction: Transaction,
    spaceId: Identifier<"space">,
    name: string,
): Promise<Identifier<"hostname">> {
    // read the hostname's account and the space's
    const [owned] = await transaction.read(hostname.table).findMany({ where: { name } });
    const served = await transaction.read(space.table).findFirst({ where: { id: spaceId } });

    // refuse an unknown hostname
    if (owned === undefined) {
        throw new CallError("NOT_FOUND", { message: `no hostname ${name}` });
    }
    // refuse a hostname of another account
    else if (owned.scope !== present(served, `space ${spaceId}`).scope) {
        throw new CallError("FORBIDDEN", {
            message: `hostname ${name} belongs to another account`,
        });
    }

    return owned.id;
}
