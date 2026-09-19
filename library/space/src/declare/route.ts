import { defineSchema, identifier, schema } from "@destack/schema";
import { Entrypoint } from "@destack/package/workload";
import { SpaceInstallationReference } from "./installation.ts";

/** A route on a domain the account is authorised to administer. */
export const SpaceRoute = defineSchema(schema.object({
    /** The registered domain; domain ownership is checked when applying the configuration. */
    domain: identifier("domain"),
    /** The absolute URL path without a query or fragment. */
    path: schema.string().regex(/^\/[^\s?#]*$/),
    /** The path matching rule. */
    match: schema.enum(["exact", "prefix"]),
    /** The application entrypoint or HTTP redirect. */
    destination: schema.union([
        schema.object({
            /** The configured or existing installation in the destination space. */
            installation: SpaceInstallationReference,
            /** The public package export serving this route. */
            entrypoint: Entrypoint,
        }),
        schema.object({
            /** The absolute HTTP destination. */
            redirect: schema.string().regex(/^https?:\/\/[^\s]+$/),
            /** The HTTP redirect response status. */
            status: schema.union([
                schema.literal(301),
                schema.literal(302),
                schema.literal(303),
                schema.literal(307),
                schema.literal(308),
            ]),
        }),
    ]),
}));
/** A source-controlled application route. */
export type SpaceRoute = schema.Infer<typeof SpaceRoute>;
