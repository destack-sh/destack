import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { release } from "@destack/registry/object";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The build of an npm package a release depends on, which the release's readers read. */
export const unit = defineObject(
    {
        name: "unit",
        plural: "units",
        title: t`${plural(argument("count"), { one: "Unit", other: "Units" })}`,
        scope: space,
        fields: {
            /** The SHA-256 digest of the unit's build manifest. */
            manifest: field.blob(),
        },
        constraints: {
            manifest: {
                kind: "check",
                condition: { manifest: { length: 64, characters: "0-9a-f" } },
            },
            unit: { kind: "unique", on: ["parentId", "manifest"] },
            build: { kind: "index", on: ["manifest"] },
        },
        access: {
            permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        },
        traits: [nested({ in: release, add: "publish", delete: "cascade" })],
    },
    (method) => ({
        create: method.create("publish", { fields: ["manifest"] }),
    }),
);

/** A persisted unit record. */
export type Unit = Select<typeof unit.table>;
