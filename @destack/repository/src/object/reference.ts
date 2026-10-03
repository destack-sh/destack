import { schema } from "@destack/schema";
import { through } from "@destack/access";
import { check, dialectSQL, sql, unique } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { repository } from "./repository.ts";
import { account } from "@destack/account/object";

/** A branch or tag of a repository's origin as last observed. */
export const reference = defineObject({
    name: "reference",
    plural: "references",
    scope: account,
    nested: { in: repository, delete: "cascade", receive: "refresh" },
    fields: {
        /** The complete name, such as refs/heads/main or refs/tags/v1. */
        name: field.string(schema.string().min(1)),
        /** The object the reference names directly: a commit, or a tag object for annotated tags. */
        object: field.string(),
        /** The commit the reference resolves to, absent when it resolves to no commit. */
        commit: field.string().optional(),
        /** When the reference was last observed at its object. */
        observedAt: field.time(),
        /** When the origin was observed without the reference, absent while it exists. */
        deletedAt: field.time().optional(),
    },
    permissions: { read: through("parent", "read") },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            fields: ["name", "object", "commit", "observedAt"],
        }),
        update: method.update(null, {
            isSystem: true,
            fields: ["object", "commit", "observedAt", "deletedAt"],
        }),
    }),
    constraints: (columns) => [
        unique("reference_parent_name").on(columns.parentId, columns.name),
        check(
            "reference_name",
            sql`${columns.name} LIKE 'refs/heads/%' OR ${columns.name} LIKE 'refs/tags/%'`,
        ),
        check(
            "reference_object",
            dialectSQL({
                sqlite: sql`length(${columns.object}) IN (40, 64) AND ${columns.object} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${columns.object}) IN (40, 64) AND (${columns.object} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        check(
            "reference_commit",
            dialectSQL({
                sqlite: sql`${columns.commit} IS NULL OR (length(${columns.commit}) IN (40, 64) AND ${columns.commit} NOT GLOB '*[^0-9a-f]*')`,
                postgresql: sql`${columns.commit} IS NULL OR (length(${columns.commit}) IN (40, 64) AND (${columns.commit} COLLATE "C") !~ '[^0-9a-f]')`,
            }),
        ),
    ],
});
