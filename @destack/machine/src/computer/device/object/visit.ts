import { intersection, principal, relation, through } from "@destack/access";
import type { ObjectClient } from "@destack/object/client";
import type { Select } from "@destack/db";
import { defineObject, device, field } from "@destack/object";
import { Identifier, schema, type Uuid } from "@destack/schema";
import { ObjectReference, type Subject } from "@destack/sync";
import { argument, plural, t } from "@destack/locale";

/** The namespace of visits' identifiers: the DNS namespace's UUIDv8 of visit.destack.app. */
const VISIT_NAMESPACE: Uuid = "a485f71a-e0ea-8189-89fa-34c9d70a0eb0";

/** An object a person opened on this computer, as browsers record visits. */
export const visit = defineObject(
    {
        name: "visit",
        plural: "visits",
        title: t`${plural(argument("count"), { one: "Visit", other: "Visits" })}`,
        scope: device,
        storage: "local",
        fields: {
            /** The person who opened it. */
            subject: field.reference(principal.user),
            /** The space whose origin opened it. */
            spaceId: field.string(schema.identifier("space")),
            /** The object. */
            object: field.json(ObjectReference),
            /** When the person last opened it. */
            openedAt: field.time(),
        },
        access: {
            // let each person signed in on this computer record their own visits
            permissions: {
                read: intersection(relation("subject"), through("device", "use")),
                delete: intersection(relation("subject"), through("device", "use")),
                record: intersection(relation("subject"), through("device", "use")),
            },
        },
    },
    (method) => ({
        create: method.create("record", { onConflict: "id" }),
        update: method.update("record", { fields: ["openedAt"] }),
        delete: method.delete("delete"),
    }),
);
/** A visit as its table stores it. */
export type Visit = Select<typeof visit.table>;

/** The visits this computer's machine records of the objects its people open. */
export const Visit = {
    /** Record that a person opened an object of a space now, one visit per person and object. */
    async record(
        objects: ObjectClient<{ readonly visit: typeof visit }>,
        subject: Subject,
        spaceId: Identifier<"space">,
        object: ObjectReference,
    ): Promise<void> {
        // name the visit by the person, the space and the object, so concurrent opens record one
        const id = await Identifier.hash(
            "visit",
            VISIT_NAMESPACE,
            `${subject.id} ${spaceId} ${ObjectReference.key(object)}`,
            objects.environment.crypto,
        );

        // open it again, or record its first visit, which a concurrent first visit answers
        const openedAt = objects.clock.now();
        const [found] = await objects.query.visit.findMany({ where: { id } });
        const written =
            found === undefined
                ? objects
                      .mutate(visit)
                      .create({ id, subjectId: subject.id, spaceId, object, openedAt })
                : objects.mutate(visit).update({ id, openedAt });
        await written.confirmed;
    },
};
