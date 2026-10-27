import { through } from "@destack/access";
import { space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Digest } from "@destack/schema";
import { release } from "./release.ts";

/** The build of an npm package a release depends on, which the release's readers read. */
export const unit = defineObject(
    {
        name: "unit",
        plural: "units",
        scope: space,
        nested: { in: release, receive: "publish", delete: "cascade" },
        fields: {
            /** The SHA-256 digest of the unit's build manifest. */
            manifest: field.string(Digest),
        },
        permissions: { read: through("parent", "read"), publish: through("parent", "publish") },
        constraints: {
            manifest: {
                kind: "check",
                condition: { manifest: { length: 64, characters: "0-9a-f" } },
            },
            unit: { kind: "unique", on: ["parentId", "manifest"] },
            build: { kind: "index", on: ["manifest"] },
        },
    },
    (method) => ({
        create: method.create("publish", { fields: ["manifest"] }),
    }),
);

/** A persisted unit record. */
export type Unit = Select<typeof unit.table>;
