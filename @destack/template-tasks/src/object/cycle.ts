import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

/** A time box in a space that tasks are planned into, as Linear's cycles. */
export const cycle = defineObject({
    name: "cycle",
    plural: "cycles",
    title: t`${plural(argument("count"), { one: "Cycle", other: "Cycles" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The cycle's name. */
        name: field.string(schema.string().min(1).max(200)),
        /** The moment the cycle starts. */
        startsAt: field.time(),
        /** The moment the cycle ends. */
        endsAt: field.time(),
    },
});
