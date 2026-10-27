import { universe } from "@destack/access";
import { and, type DatabaseConnection, eq, type Select } from "@destack/db";
import { defineObject, field, Claim, type ClaimLookup } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { installation } from "@destack/object";
import { HostnamePattern, space } from "@destack/account/object";
import type { ObjectReference } from "@destack/sync";

/** The HTTP statuses a route redirects with. */
export const REDIRECT_STATUSES = [301, 302, 303, 307, 308] as const;

/** A path on a hostname of the space's account, unique across the universe. */
export const route = defineObject(
    {
        name: "route",
        plural: "routes",
        scope: space,
        controlled: true,
        fields: {
            /** The hostname of the space's account the route serves, such as `example.com`. */
            hostname: field.string(HostnamePattern),
            /** The absolute URL path without a query or fragment. */
            path: field.string(schema.string().regex(/^\/[^\s?#]*$/u)),
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
            redirect: field.string(schema.string().regex(/^https?:\/\/[^\s]+$/u)).optional(),
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
        permissions: ["read", "write"],
    },
    (method) => ({
        create: method.create("write", { visibility: "package" }),
        get: method.get("read"),
        list: method.list("read"),
        update: method.update("write", {
            visibility: "package",
            fields: ["installationId", "view", "adapter", "redirect", "redirectStatus"],
        }),
        delete: method.delete("write", { visibility: "package" }),
    }),
);

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
        // find the most specific route of the host's hostnames and the path's prefixes in one lookup
        const candidates = Route.hostnames(host).flatMap((name) =>
            Route.paths(path).map(([candidate, match]) => [name, candidate, match]),
        );
        const found = await Claim.first(route, claims, "path", candidates);

        return found === undefined
            ? undefined
            : { ...route.reference(found.scope, found.id), id: route.identifier(found.id) };
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

    /** Read a route's destination from the columns the table's check groups together. */
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
