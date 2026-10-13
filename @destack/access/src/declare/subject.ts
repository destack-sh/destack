import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { AccessName, Subject } from "@destack/sync";
import type { PolicyDescription } from "./description.ts";

/** A type a relation accepts as subject: its objects, a subject set of them, or all of them. */
export const SubjectType = defineSchema(
    schema.object({
        /** The package declaring the subject type. */
        packageId: PackageId,
        /** The declaration-local subject type name. */
        type: AccessName,
        /** The relation whose members are the subjects, for a subject set. */
        relation: AccessName.exactOptional(),
        /** Whether one relationship relates every object of the type. */
        wildcard: schema.literal(true).exactOptional(),
    }),
);
/** A type a relation accepts as subject: its objects, a subject set of them, or all of them. */
export type SubjectType = schema.Infer<typeof SubjectType>;

/** A relation of a type, as its policy describes it. */
export type RelationDescription = PolicyDescription["relations"][string];

/** Report whether a relation accepts a subject's type, subject set and wildcard form. */
export function accepts(relation: RelationDescription, subject: Subject): boolean {
    return relation.subjects.some(
        (type) =>
            type.packageId === subject.packageId &&
            type.type === subject.type &&
            type.relation === subject.relation &&
            (type.wildcard === true) === (subject.id === "*" || subject.scope === "*"),
    );
}
