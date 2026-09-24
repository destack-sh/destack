import { z } from "zod";
import { defineSchema } from "../inspect/schema.ts";

/** The canonical lowercase UUIDv7 representation from RFC 9562. */
const UUID_V7 = "[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";

/** Define a typed entity identifier with a lowercase prefix and a UUIDv7 suffix. */
export function identifier<const Prefix extends string>(
    prefix: Prefix,
): z.core.$ZodBranded<z.ZodString, Prefix> {
    // keep prefixes literal and unambiguous before appending the UUID
    if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/.test(prefix)) {
        throw new TypeError(`Invalid identifier prefix: ${prefix}`);
    }
    const pattern = new RegExp(`^${prefix}-${UUID_V7}$(?![\\s\\S])`);

    // brand and register the validator
    const validator = z.string().regex(pattern).brand<Prefix>();
    defineSchema(validator);

    return validator as z.core.$ZodBranded<z.ZodString, Prefix>;
}
