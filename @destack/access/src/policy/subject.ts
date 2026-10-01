import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { AccessName, Subject } from "@destack/sync";

/** A type a relation accepts as subject: its objects, a subject set of them, or all of them. */
export const SubjectType = defineSchema(
    schema.object({
        /** The package declaring the subject type. */
        packageId: PackageId,
        /** The declaration-local subject type name. */
        type: AccessName,
        /** The relation whose members are the subjects, for a subject set. */
        relation: AccessName.optional(),
        /** Whether one relationship relates every object of the type. */
        wildcard: schema.literal(true).optional(),
    }),
);
/** A type a relation accepts as subject: its objects, a subject set of them, or all of them. */
export type SubjectType = schema.Infer<typeof SubjectType>;

/** A typed relationship between an object and its subjects, held by a field or by relationships. */
export interface RelationDefinition {
    /** The subject types the relation accepts. */
    readonly subjects: readonly SubjectType[];
    /** The permission to grant and revoke the relation, absent when only the system relates it. */
    readonly grantedBy?: string;
    /** Whether other types contribute themselves as subject types, as the hosts of an attachment do. */
    readonly open?: true;
}

/** Report whether a relation accepts a subject's type, subject set and wildcard form. */
export function accepts(relation: RelationDefinition, subject: Subject): boolean {
    return relation.subjects.some(
        (type) =>
            type.packageId === subject.packageId &&
            type.type === subject.type &&
            type.relation === subject.relation &&
            (type.wildcard === true) === (subject.id === "*" || subject.scope === "*"),
    );
}
