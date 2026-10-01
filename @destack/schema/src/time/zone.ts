import * as schema from "../validate/index.ts";
import { defineSchema } from "../inspect/schema.ts";

/** The longest IANA time zone name kept, beyond the longest in the database. */
const TIME_ZONE_LENGTH = 64;

/** An IANA time zone name, such as Europe/Vienna. */
export const TimeZone = Object.assign(
    defineSchema(
        schema
            .string()
            .max(TIME_ZONE_LENGTH)
            .regex(/^[A-Za-z][A-Za-z0-9_+-]*(?:\/[A-Za-z0-9_+-]+)*$/),
    ),
    {
        /** Require a time zone this runtime knows, its aliases included. */
        require(zone: TimeZone): void {
            new Intl.DateTimeFormat("en", { timeZone: zone });
        },
    },
);
/** An IANA time zone name. */
export type TimeZone = schema.Infer<typeof TimeZone>;
