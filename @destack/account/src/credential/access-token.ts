import { defineSchema, Instant, schema } from "@destack/schema";

/** A short-lived access token the universe or a space signs. */
export const AccessToken = defineSchema(
    schema.object({
        /** The signed access token. */
        accessToken: schema.string(),
        /** The HTTP authorization scheme. */
        tokenType: schema.literal("Bearer"),
        /** The exclusive expiry in epoch milliseconds. */
        expiresAt: Instant,
    }),
);
/** A short-lived access token the universe or a space signs. */
export type AccessToken = schema.Infer<typeof AccessToken>;
