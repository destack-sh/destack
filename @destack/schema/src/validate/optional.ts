import type { ZodType } from "zod";

/** Decide whether an object may leave a schema's key out, as the schema's objects decide it. */
export function isOptional(type: ZodType): boolean {
    return type._zod.optin !== undefined;
}
