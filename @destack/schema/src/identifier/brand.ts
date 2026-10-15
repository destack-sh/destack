import type { z } from "zod";

/** Brand a string validator's values by a name in types alone, as zod brands them. */
export function brand<const Name extends string>(
    validator: z.ZodString,
): z.core.$ZodBranded<z.ZodString, Name> {
    return validator as z.core.$ZodBranded<z.ZodString, Name>;
}
