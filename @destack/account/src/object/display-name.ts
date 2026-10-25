import { schema } from "@destack/schema";

/** The longest display name, in characters: a long legal name with room to spare. */
export const NAME_LENGTH = 200;

/** A display name of an account, organization, group, residency, client or token. */
export const DisplayName = schema.string().min(1).max(NAME_LENGTH);
