import { identifier, schema } from "@destack/schema";

/** Identify a space. */
export const SpaceKey = schema.object({ spaceId: identifier("space") });
