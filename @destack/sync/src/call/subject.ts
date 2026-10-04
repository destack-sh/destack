import { defineSchema, schema } from "@destack/schema";
import { NAME_PATTERN, PackageId } from "@destack/package";

/** A stable declaration-local name of an object type, relation or permission, in lowercase kebab case. */
export const AccessName = schema.string().regex(NAME_PATTERN);

/** A subject's key: its package, type, scope, identifier and relation or null. */
const SubjectKey = schema.tuple([
    schema.string(),
    schema.string(),
    schema.string(),
    schema.string(),
    schema.string().nullable(),
]);

/** The shape of a subject. */
const shape = defineSchema(
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
        relation: AccessName.exactOptional(),
    }),
);
/** A subject: a principal or other object, a subject set of its relation's members, or every object of a type. */
export type Subject = schema.Infer<typeof shape>;

/** A subject: a principal or other object, a subject set of its relation's members, or every object of a type. */
export const Subject = Object.assign(shape, {
    /** Encode a subject as a collision-free key. */
    key(subject: Subject): string {
        return JSON.stringify([
            subject.packageId,
            subject.type,
            subject.scope,
            subject.id,
            subject.relation ?? null,
        ]);
    },

    /** Read a subject back from its key. */
    read(key: string): Subject {
        const [packageId, type, scope, id, relation] = SubjectKey.parse(JSON.parse(key));

        return Subject.parse({
            packageId,
            type,
            scope,
            id,
            ...(relation === null ? {} : { relation }),
        });
    },

    /** Compare two subjects completely. */
    same(left: Subject, right: Subject): boolean {
        return Subject.key(left) === Subject.key(right);
    },
});
