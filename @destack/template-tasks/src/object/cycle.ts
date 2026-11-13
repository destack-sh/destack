import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A time box in a space that tasks are planned into. */
export const cycle = defineObject({
    name: "cycle",
    plural: "cycles",
    title: t`${plural(argument("count"), { one: "Cycle", other: "Cycles" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The cycle's number in its space, counted from 1. */
        number: field.integer(),
        /** The cycle's name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The moment the cycle starts. */
        startsAt: field.time(),
        /** The moment the cycle ends. */
        endsAt: field.time(),
    },
    constraints: {
        number: { kind: "index", on: ["number"], unique: true },
        counted: { kind: "check", condition: { number: { gte: 1 } } },
        span: { kind: "check", condition: { endsAt: { gt: { field: "startsAt" } } } },
    },
});
