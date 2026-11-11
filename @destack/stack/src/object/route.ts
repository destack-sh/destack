import { universe } from "@destack/access";
import { and, type DatabaseConnection, eq, type Select } from "@destack/db";
import { defineObject, field, Claim, type ClaimLookup } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { installation } from "@destack/object";
import { HostnamePattern, space } from "@destack/account/object";
import { ObjectReference } from "@destack/sync";
import { controlled } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The HTTP statuses a route redirects with. */
export const REDIRECT_STATUSES = [301, 302, 303, 307, 308] as const;

/** The HTTP methods a rule matches, as Gateway API's HTTPMethod lists them. */
export const ROUTE_METHODS = [
    "GET",
    "HEAD",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "OPTIONS",
    "CONNECT",
    "TRACE",
] as const;

/** An absolute URL path: `/`, or segments without a trailing slash or query. */
const RoutePath = schema.string().regex(/^\/(?:[^\s?#/]+(?:\/[^\s?#/]+)*)?$/u);

/** A header or query parameter a rule matches, by its exact value or a regular expression. */
const ValueMatch = schema.object({
    /** The header's or parameter's name, a header's compared without case. */
    name: schema.string().min(1),
    /** The value, or the pattern it matches. */
    value: schema.string(),
    /** Whether the value matches exactly or as a regular expression. */
    kind: schema.enum(["exact", "regex"]),
});

/** What a rule matches beyond the route's path, all of it at once. */
const RouteMatch = schema.object({
    /** The request's method. */
    method: schema.enum(ROUTE_METHODS).exactOptional(),
    /** The headers the request carries. */
    headers: schema.array(ValueMatch).exactOptional(),
    /** The query parameters the request carries. */
    query: schema.array(ValueMatch).exactOptional(),
});

/** How a filter replaces a path: whole, or the part the route's prefix matched. */
const PathModifier = schema.object({
    /** Whether the value replaces the whole path or the matched prefix. */
    kind: schema.enum(["full", "prefix"]),
    /** The replacing path. */
    value: RoutePath,
});

/** The headers a filter sets, adds and removes, Gateway API's HTTPHeaderFilter. */
const HeaderModifier = {
    /** The headers replacing any with their names. */
    set: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The headers appended beside any with their names. */
    add: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The names of the headers removed. */
    remove: schema.array(schema.string()).exactOptional(),
};

/** A change a rule makes to the request or its response, applied in order. */
const RouteFilter = schema.discriminatedUnion("kind", [
    schema.object({
        /** Redirect to the request's URL with these parts replaced. */
        kind: schema.literal("redirect"),
        /** The scheme. */
        scheme: schema.enum(["http", "https"]).exactOptional(),
        /** The hostname. */
        hostname: HostnamePattern.exactOptional(),
        /** The path. */
        path: PathModifier.exactOptional(),
        /** The redirect status. */
        status: schema.union(REDIRECT_STATUSES.map((status) => schema.literal(status))),
    }),
    schema.object({
        /** Hand the entry the request at another path. */
        kind: schema.literal("rewrite"),
        /** The path. */
        path: PathModifier,
    }),
    schema.object({
        /** Change the request's headers before the entry reads them. */
        kind: schema.literal("request-headers"),
        ...HeaderModifier,
    }),
    schema.object({
        /** Change the response's headers before the client reads them. */
        kind: schema.literal("response-headers"),
        ...HeaderModifier,
    }),
]);

/** What serves a rule's requests: an entry of the route's installation, or another site. */
const RouteEntry = schema.discriminatedUnion("kind", [
    schema.object({
        /** The installation's whole origin, served below the route's path. */
        kind: schema.literal("origin"),
    }),
    schema.object({
        /** A view the installed package declares, served below the route's path. */
        kind: schema.literal("view"),
        /** The view. */
        view: DeclarationName,
        /** The object the view opens for, as its target. */
        object: ObjectReference.exactOptional(),
    }),
    schema.object({
        /** An adapter the installed package declares. */
        kind: schema.literal("adapter"),
        /** The adapter. */
        adapter: DeclarationName,
    }),
    schema.object({
        /** Another site, its origin fixed and the path below the route's appended. */
        kind: schema.literal("url"),
        /** The site's absolute URL. */
        url: schema.string().regex(/^https?:\/\/[^\s]+$/u),
    }),
]);

/** One rule of a route: what it matches, how it changes the request, and what serves it. */
export const RouteRule = schema.object({
    /** What the request matches beyond the route's path, any request without. */
    matches: RouteMatch.exactOptional(),
    /** The filters, applied in order. */
    filters: schema.array(RouteFilter).exactOptional(),
    /** What serves the request, absent for a redirect. */
    entry: RouteEntry.exactOptional(),
});

/** One rule of a route. */
export type RouteRule = schema.Infer<typeof RouteRule>;

/** A path on a hostname of the space's account, unique across the universe, and its rules. */
export const route = defineObject(
    {
        name: "route",
        plural: "routes",
        title: t`${plural(argument("count"), { one: "Route", other: "Routes" })}`,
        scope: space,
        fields: {
            /** The hostname of the space's account the route serves, such as `example.com`. */
            hostname: field.string(HostnamePattern),
            /** The absolute URL path: `/`, or segments without a trailing slash or query. */
            path: field.string(RoutePath),
            /** Whether the path matches exactly or as a prefix. */
            match: field.enum(["exact", "prefix"]),
            /** The installation whose entries the rules serve, absent when none serves one. */
            installation: field.reference(installation, { delete: "restrict" }).optional(),
            /** The rules, the first matching one serving a request. */
            rules: field.json(schema.array(RouteRule).min(1)),
        },
        indexes: {
            path: { on: ["hostname", "path", "match"], across: () => universe },
        },
        constraints: {
            path: { kind: "check", condition: { path: { startsWith: "/" } } },
        },
        access: {
            permissions: ["read", "write"],
        },
        traits: [controlled()],
    },
    (method) => ({
        create: method.create("write"),
        get: method.get("read"),
        list: method.list("read"),
        update: method.update("write", { fields: ["installationId", "rules"] }),
        delete: method.delete("write"),
    }),
);

/** A persisted route record. */
export type Route = Select<typeof route.table>;

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

    /** Select the rule serving a request: the most specific matching one, the first of equals. */
    rule(row: Pick<Route, "rules">, request: Request): RouteRule | undefined {
        // keep the rules the request matches, each with its specificity
        const url = new URL(request.url);
        const matching = row.rules.flatMap((rule, index) =>
            Route.matches(rule, request, url) ? [{ rule, index }] : [],
        );

        // take the most specific: a method, then more headers, then more query parameters, then the first
        const ranked = matching.toSorted(
            (left, right) =>
                Number(right.rule.matches?.method !== undefined) -
                    Number(left.rule.matches?.method !== undefined) ||
                (right.rule.matches?.headers?.length ?? 0) -
                    (left.rule.matches?.headers?.length ?? 0) ||
                (right.rule.matches?.query?.length ?? 0) -
                    (left.rule.matches?.query?.length ?? 0) ||
                left.index - right.index,
        );

        return ranked[0]?.rule;
    },

    /** Report whether a request matches a rule's method, headers and query parameters. */
    matches(rule: RouteRule, request: Request, url: URL): boolean {
        // compare the method, then each header and query parameter by its exact text or its pattern
        const { matches } = rule;

        return (
            (matches?.method === undefined || matches.method === request.method) &&
            (matches?.headers ?? []).every((each) =>
                Route.value(each, request.headers.get(each.name)),
            ) &&
            (matches?.query ?? []).every((each) =>
                Route.value(each, url.searchParams.get(each.name)),
            )
        );
    },

    /** Report whether a present value matches a header's or parameter's exact text or pattern. */
    value(
        match: { readonly value: string; readonly kind: "exact" | "regex" },
        actual: string | null,
    ): boolean {
        return (
            actual !== null &&
            (match.kind === "exact"
                ? actual === match.value
                : Route.pattern(match.value)?.test(actual) === true)
        );
    },

    /** Compile a match's pattern, absent for an invalid one. */
    pattern(text: string): RegExp | undefined {
        try {
            return new RegExp(text, "u");
        } catch {
            return undefined;
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
