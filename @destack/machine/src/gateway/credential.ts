import { schema } from "@destack/schema";

/** The credential a person's page acts within: the session they signed in with, or their client. */
export const PageCredential = schema.union([
    schema.identifier("session"),
    schema.identifier("client"),
]);
/** The credential a person's page acts within. */
export type PageCredential = schema.Infer<typeof PageCredential>;
