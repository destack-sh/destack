import { none } from "@destack/access";
import { foreignKey, unique } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { resource, space } from "@destack/space/object";

/** A space's bucket, the facet of a bucket resource with files. */
export const bucket = defineObject({
    name: "bucket",
    identity: "resource",
    plural: "buckets",
    scope: space,
    fields: {
        /** The fixed resource kind, which the underlying resource must have. */
        kind: field.enum(["bucket"]).default("bucket"),
    },
    constraints: (entry) => [
        unique("bucket_scope_id").on(entry.scope, entry.id),
        foreignKey({
            columns: [entry.scope, entry.id, entry.kind],
            foreignColumns: [resource.table.scope, resource.table.id, resource.table.kind],
        }).onDelete("restrict"),
    ],
    permissions: {
        get: none(),
        list: none(),

        files: none(),
        download: none(),
        upload: none(),
        remove: none(),
    },
    administration: ["get", "list"],
    methods: {
        get: method.get("get"),
        list: method.list("list"),
        create: method.create(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    },
});
