import { defineSchema, identifier, schema } from "@destack/schema";

/** The service authorized to change a space's administrative records. */
export const SpaceAuthority = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Administration by a local or self-hosted daemon. */
            kind: schema.literal("host"),
            /** The administrative host, independently of execution placement. */
            hostId: identifier("host"),
        }),
        schema.object({
            /** Administration by the regional space service. */
            kind: schema.literal("region"),
            /** The region holding the authoritative administration database. */
            regionId: identifier("region"),
        }),
    ]),
);

/** A local or regional administrative authority. */
export type SpaceAuthority = schema.Infer<typeof SpaceAuthority>;
