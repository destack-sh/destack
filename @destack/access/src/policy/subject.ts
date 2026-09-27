import { defineSchema, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { AccessName } from "./expression.ts";

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
    /** The permission whose holders grant and revoke the relation, absent when only the system relates it. */
    readonly grantedBy?: string;
    /** Whether other types contribute themselves as subject types, as the hosts of an attachment do. */
    readonly open?: true;
}

/** A subject: a principal or other object, a subject set of its relation's members, or every object of a type. */
export const Subject = defineSchema(
    schema.object({
        /** The package declaring the subject type. */
        packageId: PackageId,
        /** The declaration-local subject type name. */
        type: AccessName,
        /** The scope containing the subject, or `*` for every scope. */
        scope: schema.string().min(1),
        /** The subject identifier, or `*` for every object of the type in the scope. */
        id: schema.string().min(1),
        /** The relation whose members are the subjects, for a subject set. */
        relation: AccessName.optional(),
    }),
);
/** A subject: a principal or other object, a subject set of its relation's members, or every object of a type. */
export type Subject = schema.Infer<typeof Subject>;

/** Construct a collision-free subject key. */
export function subjectKey(subject: Subject): string {
    return JSON.stringify([
        subject.packageId,
        subject.type,
        subject.scope,
        subject.id,
        subject.relation ?? null,
    ]);
}

/** Read a subject back from its key. */
export function keySubject(key: string): Subject {
    const [packageId, type, scope, id, relation] = JSON.parse(key) as [
        string,
        string,
        string,
        string,
        string | null,
    ];

    return Subject.parse({
        packageId,
        type,
        scope,
        id,
        ...(relation === null ? {} : { relation }),
    });
}

/** Compare two subjects completely. */
export function sameSubject(left: Subject, right: Subject): boolean {
    return subjectKey(left) === subjectKey(right);
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
