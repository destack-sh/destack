import { v7 } from "uuid";
import { z } from "zod";
import { defineSchema } from "../declare/schema.ts";
import { brand } from "./brand.ts";

/** The characters of a UUID's canonical text form, RFC 9562. */
const UUID_LENGTH = 36;

/** The position of a UUID's version digit in its text form (RFC 9562 4.2). */
const VERSION_POSITION = 14;

/** The canonical lowercase UUIDv7 representation from RFC 9562. */
const UUID_V7 = "[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";

/** The text of a UUID's bytes 11 to 13, random bits after a UUIDv7's counter (RFC 9562 5.7). */
const POSITION_RANGE = { start: 26, end: 32 } as const;

/** The positions bytes 11 to 13 take: 2^24. */
const POSITION_LIMIT = 2 ** 24;

/** The identifier validators by prefix, each built and registered once. */
const VALIDATORS = new Map<string, z.ZodString>();

/** A prefixed UUIDv7 identifier. */
export type Identifier<Prefix extends string> = z.output<ReturnType<typeof identifier<Prefix>>>;

/** Define a typed entity identifier with a lowercase prefix and a UUIDv7 suffix. */
export function identifier<const Prefix extends string>(
    prefix: Prefix,
): z.core.$ZodBranded<z.ZodString, Prefix> {
    // reuse the prefix's validator
    const known = VALIDATORS.get(prefix);
    if (known !== undefined) {
        return brand<Prefix>(known);
    }

    // keep prefixes literal and unambiguous before appending the UUID
    if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u.test(prefix)) {
        throw new TypeError(`invalid identifier prefix: ${prefix}`);
    }

    // match the prefix followed by a UUIDv7, and register and keep the validator
    const validator = defineSchema(z.string().regex(new RegExp(`^${prefix}-${UUID_V7}$`, "u")));
    VALIDATORS.set(prefix, validator);

    return brand<Prefix>(validator);
}

/** Define an identifier of any entity kind: any lowercase prefix and a UUIDv7 suffix. */
export function anyIdentifier(): z.core.$ZodBranded<z.ZodString, string> {
    // match any valid prefix followed by a UUIDv7
    const pattern = new RegExp(`^[a-z][a-z0-9]*(?:-[a-z0-9]+)*-${UUID_V7}$`, "u");

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

    /** Derive a request object's identifier: the request's time and bits as a UUIDv7. */
    derive<const Prefix extends string>(
        prefix: Prefix,
        requestId: string,
        position?: number,
    ): Identifier<Prefix> {
        // mark the request's bits as a UUIDv7
        const head = requestId.slice(0, VERSION_POSITION);
        const tail = requestId.slice(VERSION_POSITION + 1);
        const uuid = `${head}7${tail}`;
        if (position === undefined) {
            return identifier(prefix).parse(`${prefix}-${uuid}`);
        }

        // write the position over bytes 11 to 13
        if (!Number.isInteger(position) || position < 0 || position >= POSITION_LIMIT) {
            throw new RangeError(`identifier position ${position} is outside 0 to 2^24`);
        }
        const width = POSITION_RANGE.end - POSITION_RANGE.start;
        const written = position.toString(16).padStart(width, "0");
        const positioned = `${uuid.slice(0, POSITION_RANGE.start)}${written}${uuid.slice(POSITION_RANGE.end)}`;

        return identifier(prefix).parse(`${prefix}-${positioned}`);
    },

    /** Keep the values that are identifiers of a prefix, in their order. */
    filter<const Prefix extends string>(
        prefix: Prefix,
        values: readonly unknown[],
    ): Identifier<Prefix>[] {
        // parse each value by the prefix's validator
        const validator = identifier(prefix);

        return values.flatMap((value) => {
            const parsed = validator.safeParse(value);

            return parsed.success ? [parsed.data] : [];
        });
    },

    /** Read the UUIDv7 an identifier ends with. */
    uuid(value: Identifier<string>): string {
        return value.slice(-UUID_LENGTH);
    },
};
