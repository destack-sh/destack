import { Condition } from "@destack/db";
import type { ObjectType } from "@destack/object";
import { ObjectTypeReference } from "@destack/sync";

/** What a waiting installation waits for: a row of one object type meeting a condition. */
export type InstallationWait = {
    /** The object type waited for, by its reference key. */
    readonly waitType: string;
    /** The condition one of that type's rows meets once the installation reconciles again. */
    readonly waitCondition: Condition;
};

/** The waits of installations, as the fields their controller's watches match. */
export const InstallationWait = {
    /** The schema of the condition, a stored query over the waited type's rows. */
    condition: Condition.schema,

    /** The fields of an installation that waits for nothing. */
    none: { waitType: null, waitCondition: null },

    /** Wait for a row of an object type meeting a condition. */
    of(object: ObjectType, condition: Condition): InstallationWait {
        return {
            waitType: ObjectTypeReference.key(object.typeReference),
            waitCondition: condition,
        };
    },
};
