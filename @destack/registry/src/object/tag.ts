import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, defineShape, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { packageObject, TagName } from "./package.ts";
import { release } from "./release.ts";

/** An npm distribution tag of a package, such as latest, pointing at one published version. */
export const tag = defineObject(
    {
        name: "tag",
        plural: "tags",
        scope: space,
        nested: { in: packageObject, receive: "publish", delete: "restrict" },
        fields: {
            /** The npm distribution tag name, invalid as a version range. */
            name: field.string(TagName),
            /** The selected published version. */
            version: field.string(),
        },
        permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        constraints: {
            name: { kind: "unique", on: ["parentId", "name"] },
            release: {
                kind: "foreignKey",
                on: ["parentId", "version"],
                references: { object: () => release, on: ["parentId", "version"] },
                onDelete: "no action",
            },
        },
    },
    (method) => ({
        create: method.create("publish", { fields: ["name", "version"] }),
        update: method.update("publish", { fields: ["version"] }),
        delete: method.delete("publish"),
    }),
);

/** A persisted tag record. */
export type Tag = Select<typeof tag.table>;

/** The shape replicating a package's distribution tag from its publisher's forge home. */
export const tagShape = defineShape({
    name: "tag",
    parameters: schema.object({
        /** The package. */
        package: PackageId,
        /** The distribution tag, such as latest. */
        name: TagName,
    }),
    objects: [tag],
    queries: ({ package: parentId, name }) => [tag.query({ where: { parentId, name } })],
});
