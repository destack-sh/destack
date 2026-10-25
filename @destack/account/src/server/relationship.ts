import { AccessError, Relationship } from "@destack/access";
import { type MethodContext, relationship } from "@destack/object";
import type { ObjectReference, Subject } from "@destack/sync";

/** One relationship of a subject to an object through a relation. */
export interface RelatedFields {
    /** The related object. */
    readonly object: ObjectReference;
    /** The relation. */
    readonly relation: string;
    /** The related subject, a principal or a subject set. */
    readonly subject: Subject;
}

/** The relationships the store writes as itself. */
export const StoreRelationship = {
    /** Relate a subject to an object, keeping the same relationship it has. */
    async touch(context: MethodContext, fields: RelatedFields): Promise<void> {
        await context.mutate(context.self, [
            relationship.call("touch", { ...relationship.in(fields.object.scope), ...fields }),
        ]);
    },

    /** Delete a subject's relationship to an object, refusing one it lacks. */
    async delete(context: MethodContext, fields: RelatedFields): Promise<void> {
        // find the relationship
        const { object, relation, subject } = fields;
        const [related] = await Relationship.readOn(context, object, { relation, subject });
        if (related === undefined) {
            throw AccessError.of(
                "RELATIONSHIP_NOT_FOUND",
                `${subject.type} ${subject.id} has no ${relation} relationship to ${object.type} ${object.id}`,
            );
        }

        // delete it as the store
        await context.mutate(context.self, [
            relationship.call("delete", { ...relationship.in(related.scope), id: related.id }),
        ]);
    },
};
