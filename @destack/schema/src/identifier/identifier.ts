import { v7 } from "uuid";
import { z } from "zod";
import { defineSchema } from "../inspect/schema.ts";

/** The characters of a UUID's canonical text form, RFC 9562. */
const UUID_LENGTH = 36;

/** The canonical lowercase UUIDv7 representation from RFC 9562. */
const UUID_V7 = "[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";

/** The identifier validators by prefix, built once each. */
const VALIDATORS = new Map<string, z.core.$ZodBranded<z.ZodString, string>>();

/** A prefixed UUIDv7 identifier. */
export type Identifier<Prefix extends string> = z.output<ReturnType<typeof identifier<Prefix>>>;

/** Define a typed entity identifier with a lowercase prefix and a UUIDv7 suffix. */
export function identifier<const Prefix extends string>(
    prefix: Prefix,
): z.core.$ZodBranded<z.ZodString, Prefix>;
/** Define an identifier validator, whose signature above brands the prefix it reads. */
export function identifier(prefix: string): z.core.$ZodBranded<z.ZodString, string> {
    // reuse the prefix's validator
    const known = VALIDATORS.get(prefix);
    if (known !== undefined) {
        return known;
    }

    // keep prefixes literal and unambiguous before appending the UUID
    if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/u.test(prefix)) {
        throw new TypeError(`invalid identifier prefix: ${prefix}`);
    }

    // match the prefix followed by a UUIDv7
    const pattern = new RegExp(`^${prefix}-${UUID_V7}$(?![\\s\\S])`, "u");

    // brand, register and keep the validator
    const validator = z.string().regex(pattern).brand<string>();
    defineSchema(validator);
    VALIDATORS.set(prefix, validator);

    return validator;
}

/** Define an identifier of any entity kind: any lowercase prefix and a UUIDv7 suffix. */
export function anyIdentifier(): z.core.$ZodBranded<z.ZodString, string> {
    // match any valid prefix followed by a UUIDv7
    const pattern = new RegExp(`^[a-z][a-z0-9]*(?:-[a-z0-9]+)*-${UUID_V7}$(?![\\s\\S])`, "u");

    // brand and register the validator
    const validator = z.string().regex(pattern).brand<string>();
    defineSchema(validator);

    return validator;
}

/** Create and read prefixed identifiers. */
export const Identifier = {
    /** Create a new identifier with a prefix, ordered by its creation time. */
    create<const Prefix extends string>(prefix: Prefix): Identifier<Prefix> {
        return identifier(prefix).parse(`${prefix}-${v7()}`);
    },

    /** Read the UUIDv7 an identifier ends with. */
    uuid(value: Identifier<string>): string {
        return value.slice(-UUID_LENGTH);
    },
};
