import { universe } from "@destack/access";
import { and, type DatabaseConnection, eq, type Select } from "@destack/db";
import { defineObject, field, Claim, type ClaimLookup } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { REDIRECT_STATUSES, RouteDefinition } from "../declare/route.ts";
import { installation } from "./installation.ts";
import { hostname, space } from "@destack/account/object";
import type { ObjectReference } from "@destack/sync";

/** A path on a hostname of the space's account, unique across the universe. */
export const route = defineObject({
    name: "route",
    plural: "routes",
    scope: space,
    declarable: { schema: RouteDefinition },
    // NOTE #Incomplete: provision each route's hostname at Cloudflare for SaaS and report it served in the edge's route reconciler (D4)
    controlled: true,
    fields: {
        /** The hostname of the space's account the route serves. */
        hostname: field.string(schema.identifier("hostname")),
        /** The absolute URL path. */
        path: field.string(),
        /** Whether the path matches exactly or as a prefix. */
        match: field.enum(["exact", "prefix"]),
        // the installation destinations
        /** The serving installation. */
        installation: field.reference(installation, { delete: "restrict" }).optional(),
        /** The view the installed package declares. */
        view: field.string(DeclarationName).optional(),
        /** The adapter the installed package declares. */
        adapter: field.string(DeclarationName).optional(),

        // the redirect destination
        /** The absolute HTTP destination. */
        redirect: field.string().optional(),
        /** The HTTP redirect status. */
        redirectStatus: field.integer().optional(),
    },
    indexes: {
        path: { on: ["hostname", "path", "match"], across: () => universe },
    },
    constraints: {
        path: { kind: "check", condition: { path: { startsWith: "/" } } },
        destination: {
            kind: "check",
            condition: {
                OR: [
                    {
                        installationId: { isNotNull: true },
                        redirect: { isNull: true },
                        redirectStatus: { isNull: true },
                        OR: [
                            { view: { isNull: true }, adapter: { isNotNull: true } },
                            { view: { isNotNull: true }, adapter: { isNull: true } },
                        ],
                    },
                    {
                        installationId: { isNull: true },
                        view: { isNull: true },
                        adapter: { isNull: true },
                        redirect: { isNotNull: true },
                        redirectStatus: { in: [...REDIRECT_STATUSES] },
                    },
                ],
            },
        },
    },
}).methods();

/** A persisted route record. */
export type Route = Select<typeof route.table>;

/** Where a route sends its requests: an installation's view or adapter, or a redirect. */
export type RouteDestination =
    | {
          readonly kind: "view";
          readonly installationId: Identifier<"installation">;
          readonly view: string;
      }
    | {
          readonly kind: "adapter";
          readonly installationId: Identifier<"installation">;
          readonly adapter: string;
      }
    | { readonly kind: "redirect"; readonly location: string; readonly status: number };

/** The routes of the universe's hostnames, as a front door finds them. */
export const Route = {
    /** Find the route serving a hostname's path through the owners of the universe's names. */
    async locate(
        claims: ClaimLookup,
        host: string,
        path: string,
    ): Promise<(ObjectReference & { readonly id: Identifier<"route"> }) | undefined> {
        // try each hostname the host matches, most specific first
        // NOTE #Performance: a request costs a claim lookup per candidate until the edge keeps routes by hostname in Workers KV (D4)
        for (const name of Route.hostnames(host)) {
            const served = await Claim.lookup(hostname, claims, "name", [name]);
            if (served === undefined) {
                continue;
            }

            // try the exact path, then each prefix, longest first
            for (const [candidate, match] of Route.paths(path)) {
                const found = await Claim.lookup(route, claims, "path", [
                    served.id,
                    candidate,
                    match,
                ]);
                if (found !== undefined) {
                    return {
                        ...route.reference(found.scope, found.id),
                        id: route.identifier(found.id),
                    };
                }
            }
        }

        return undefined;
    },

    /** List the hostnames a host matches, most specific first. */
    hostnames(host: string): readonly string[] {
        const labels = host.split(".");
        const wildcards = labels
            .slice(1, -1)
            .map((_, index) => `*.${labels.slice(index + 1).join(".")}`);

        return [host, ...wildcards];
    },

    /** List the paths a path matches, most specific first. */
    paths(path: string): readonly (readonly [string, "exact" | "prefix"])[] {
        // take each parent path once, ending at the root
        const segments = path.split("/").slice(1);
        const prefixes = segments.map(
            (_, index) => `/${segments.slice(0, segments.length - index).join("/")}`,
        );
        const parents = [...new Set([...prefixes, "/"])];

        return [[path, "exact"], ...parents.map((parent) => [parent, "prefix"] as const)];
    },

    /** Read a route's destination from the columns the table's check keeps together. */
    destination(row: Route): RouteDestination {
        const { installationId, view, adapter, redirect, redirectStatus } = row;

        // read a redirect
        if (redirect !== null && redirectStatus !== null) {
            return { kind: "redirect", location: redirect, status: redirectStatus };
        }
        // read an installation's view
        else if (installationId !== null && view !== null) {
            return { kind: "view", installationId, view };
        }
        // read an installation's adapter
        else if (installationId !== null && adapter !== null) {
            return { kind: "adapter", installationId, adapter };
        }
        // refuse a row the table's check forbids
        else {
            throw new TypeError(`route ${row.id} has no destination`);
        }
    },

    /** Read a route of a space by identifier, absent for none. */
    async find(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        id: Identifier<"route">,
    ): Promise<Route | undefined> {
        const [found] = await database
            .select()
            .from(route.table)
            .where(and(eq(route.table.id, id), route.inScope(spaceId)));

        return found;
    },
};
