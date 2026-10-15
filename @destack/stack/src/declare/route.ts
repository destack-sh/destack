import { HostnamePattern } from "@destack/account/object";
import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { InstallationReference } from "./installation.ts";

/** The HTTP statuses a route redirects with. */
export const REDIRECT_STATUSES = [301, 302, 303, 307, 308] as const;

/** A path on a hostname of the space's account, served by an installation or redirected. */
export const RouteDefinition = defineSchema(
    schema.object({
        /** The hostname of the space's account the route serves, or its wildcard. */
        hostname: HostnamePattern,
        /** The absolute URL path without a query or fragment. */
        path: schema.string().regex(/^\/[^\s?#]*$/u),
        /** The path matching rule. */
        match: schema.enum(["exact", "prefix"]),
        /** The installation's view, its adapter, or an HTTP redirect. */
        destination: schema.union([
            schema.object({
                /** The configured or existing installation in the destination space. */
                installation: InstallationReference,
                /** The view the installed package declares. */
                view: DeclarationName,
            }),
            schema.object({
                /** The configured or existing installation in the destination space. */
                installation: InstallationReference,
                /** The adapter the installed package declares, answering the requests itself. */
                adapter: DeclarationName,
            }),
            schema.object({
                /** The absolute HTTP destination. */
                redirect: schema.string().regex(/^https?:\/\/[^\s]+$/u),
                /** The HTTP redirect response status. */
                status: schema.literal(REDIRECT_STATUSES),
            }),
        ]),
    }),
);

/** A path on a hostname of the space's account, served by an installation or redirected. */
export type RouteDefinition = schema.Infer<typeof RouteDefinition>;
