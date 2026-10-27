import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, defineShape, field } from "@destack/object";
import { Commit, schema } from "@destack/schema";
import { repository } from "./repository.ts";

/** A branch or tag of a repository's origin as last observed. */
export const reference = defineObject(
    {
        name: "reference",
        plural: "references",
        scope: space,
        nested: { in: repository, delete: "cascade", receive: "record" },
        fields: {
            /** The complete name, such as refs/heads/main or refs/tags/v1. */
            name: field.string(schema.string().min(1)),
            /** The object the reference points at directly. */
            object: field.string(),
            /** The commit the reference resolves to, absent when it resolves to no commit. */
            commit: field.string(Commit).optional(),
            /** When the reference was last observed at its object. */
            observedAt: field.time(),
            /** When the origin was observed without the reference, absent while it exists. */
            deletedAt: field.time().optional(),
        },
        // the repository's refreshers and reporters record its references
        permissions: { read: through("parent", "read"), record: through("parent", "record") },
        constraints: {
            parentName: { kind: "unique", on: ["parentId", "name"] },
            name: {
                kind: "check",
                condition: {
                    name: { OR: [{ startsWith: "refs/heads/" }, { startsWith: "refs/tags/" }] },
                },
            },
            object: {
                kind: "check",
                condition: { object: { length: { in: [40, 64] }, characters: "0-9a-f" } },
            },
            commit: {
                kind: "check",
                condition: {
                    commit: {
                        OR: [{ isNull: true }, { length: { in: [40, 64] }, characters: "0-9a-f" }],
                    },
                },
            },
        },
    },
    (method) => ({
        create: method.create("record", {
            visibility: "package",
            fields: ["name", "object", "commit", "observedAt"],
        }),
        createMany: method.createMany("record", {
            visibility: "package",
            fields: ["name", "object", "commit", "observedAt"],
        }),
        update: method.update("record", {
            visibility: "package",
            fields: ["object", "commit", "observedAt", "deletedAt"],
        }),
        updateMany: method.updateItems("record", {
            visibility: "package",
            fields: ["object", "commit", "observedAt", "deletedAt"],
        }),
    }),
);

/** The shape replicating one reference of a repository, stored in the forge's residency home. */
export const referenceShape = defineShape({
    name: "reference",
    parameters: schema.object({
        /** The repository. */
        repository: schema.identifier("repository"),
        /** The reference's complete name, such as `refs/heads/main`. */
        name: schema.string().min(1),
    }),
    objects: [reference],
    queries: ({ repository: parentId, name }) => [reference.query({ where: { parentId, name } })],
});
