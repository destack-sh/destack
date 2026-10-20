import { Account, space, user } from "@destack/account/object";
import { Follow, relationship } from "@destack/object";
import type { RowImage } from "@destack/db";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { accountSpaceShape, activityShape, relationshipShape } from "../object/activity.ts";

/** Follow a home's person's relationships and activities in related spaces, on their behalf. */
export const HOME_FOLLOWS: readonly Follow[] = [
    space.follow(relationshipShape, {
        key: async (row, view, home) => {
            // follow the person's relationships where the store's own space is their home
            const person =
                home === undefined || row.id !== home
                    ? undefined
                    : await Account.homeOwner(view, home);

            return person === undefined
                ? undefined
                : {
                      key: { scope: Scope.universe.id, person: user.identifier(person.id) },
                      subject: person,
                  };
        },
    }),
    relationship.follow(activityShape, {
        // follow the person's activity elsewhere at best effort, never gating the home
        isReadinessGate: false,
        key: async (row, view, home) => {
            // follow nothing outside a home, or for another principal's relationship
            const person = home === undefined ? undefined : await Account.homeOwner(view, home);
            if (person === undefined || row.subjectId !== person.id) {
                return undefined;
            }

            // map a relationship on a space, or on an object in one, to the space
            const related = relatedSpace(row);

            return related === undefined || related === home
                ? undefined
                : { key: { scope: related, recipient: person.id }, subject: person };
        },
    }),
    relationship.follow(accountSpaceShape, {
        // follow the person's accounts' spaces at best effort, never gating the home
        isReadinessGate: false,
        key: async (row, view, home) => {
            // follow the spaces of an account the person relates to, decided for the person
            const person = home === undefined ? undefined : await Account.homeOwner(view, home);
            const related = schema.identifier("account").safeParse(row.objectId);
            if (person === undefined || row.subjectId !== person.id || !related.success) {
                return undefined;
            }

            return {
                key: { scope: Scope.universe.id, account: related.data },
                subject: person,
            };
        },
    }),
    space.follow(activityShape, {
        // follow the person's activity elsewhere at best effort, never gating the home
        isReadinessGate: false,
        key: async (row, view, home) => {
            // follow nothing outside a home, nor in the home itself
            const person = home === undefined ? undefined : await Account.homeOwner(view, home);
            if (person === undefined || row.id === home) {
                return undefined;
            }

            // follow a space of an account the person relates to
            const related = await view.read(relationship.table).findMany({
                where: { objectId: row.scope, subjectId: person.id },
            });

            return related.length === 0
                ? undefined
                : { key: { scope: row.id, recipient: person.id }, subject: person };
        },
    }),
];

/** Read the space a relationship's object is or lives in, absent outside spaces. */
function relatedSpace(
    row: Pick<RowImage<typeof relationship.table>, "objectId" | "objectScope">,
): string | undefined {
    // take a space object, else a space its object lives in
    const { objectId, objectScope } = row;
    if (space.identifies(objectId)) {
        return objectId;
    } else if (space.identifies(objectScope)) {
        return objectScope;
    }

    return undefined;
}
