import { hostname } from "@destack/account/object";
import { type MethodContext } from "@destack/object";
import { Route, route, type RouteRule } from "../../object/index.ts";
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
                requireRules(call.input.installationId !== undefined, call.input.rules);

                return next();
            },
            update: async (call, next) => {
                // refuse rules that cannot serve, such as an entry of an installation the route lacks
                const current = await call.read(route).findFirst({ where: { id: call.target.id } });
                const installationId =
                    call.input.installationId === undefined
                        ? current?.installationId
                        : call.input.installationId;
                requireRules(
                    installationId !== undefined && installationId !== null,
                    call.input.rules ?? current?.rules ?? [],
                );

                return next();
            },
        },
    });
}

/** Require rules that can serve: one redirect or entry each, valid patterns, an installation. */
function requireRules(hasInstallation: boolean, rules: readonly RouteRule[]): void {
    for (const rule of rules) {
        // require exactly one of a redirect and an entry
        const isRedirect = (rule.filters ?? []).some((filter) => filter.kind === "redirect");
        if (isRedirect === (rule.entry !== undefined)) {
            throw StackError.of("ROUTE_INVALID", "a route's rule either redirects or has an entry");
        }

        // require patterns that compile
        const values = [...(rule.matches?.headers ?? []), ...(rule.matches?.query ?? [])];
        const invalid = values.find(
            (each) => each.kind === "regex" && Route.pattern(each.value) === undefined,
        );
        if (invalid !== undefined) {
            throw StackError.of(
                "ROUTE_INVALID",
                `invalid pattern for ${invalid.name}: ${invalid.value}`,
            );
        }

        // require the installation an installation's entry is served from
        if (rule.entry !== undefined && rule.entry.kind !== "url" && !hasInstallation) {
            throw StackError.of(
                "ROUTE_INVALID",
                `a route serving an installation's ${rule.entry.kind} needs its installation`,
            );
        }
    }
}

/** Require a hostname of the space's account by name, the only hostnames its store follows. */
async function requireHostname(call: MethodContext, name: string): Promise<void> {
    // refuse a hostname the space's account lacks
    const [owned] = await call.read(hostname).findMany({ where: { name } });
    if (owned === undefined) {
        throw StackError.of("HOSTNAME_NOT_FOUND", `hostname ${name} not found`);
    }
}
