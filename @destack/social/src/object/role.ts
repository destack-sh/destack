import { relation, union } from "@destack/access";
import { field, type ObjectDefinition } from "@destack/object";

/** The subject types a relation accepts. */
type Subjects = NonNullable<ObjectDefinition["relations"]>[string]["subjects"];

/** Build the standard owner, editor, commenter and viewer roles a host spreads into its definition. */
export function hostRoles(subjects: Subjects) {
    return {
        fields: {
            /** The principal who created and manages the object. */
            owner: field.subject().caller(),
        },
        relations: {
            editor: { subjects },
            commenter: { subjects },
            viewer: { subjects },
        },
        permissions: {
            read: union(
                relation("owner"),
                relation("editor"),
                relation("commenter"),
                relation("viewer"),
            ),
            comment: union(relation("owner"), relation("editor"), relation("commenter")),
            edit: union(relation("owner"), relation("editor")),
            manage: relation("owner"),
        },
        grantedBy: "manage",
    } as const;
}
