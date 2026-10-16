import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
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
