import type { Select } from "@destack/db";
import { space } from "@destack/account/object";
import { through } from "@destack/access";
import { defineObject, field, recoverable } from "@destack/object";
import { PackageId } from "@destack/package";
import { argument, plural, t } from "@destack/locale";

/** A package's tables the space's main database keeps once no installation declares them. */
export const orphan = defineObject(
    {
        name: "orphan",
        plural: "orphans",
        title: t`${plural(argument("count"), { one: "Orphan", other: "Orphans" })}`,
        scope: space,
        fields: {
            /** The package whose tables the space's main database keeps. */
            packageId: field.string(PackageId),
        },
        access: {
            permissions: {
                // let the space's readers see what its database keeps, and its managers delete it
                read: through("scope", "read"),
                list: through("scope", "read"),
                manage: through("scope", "manage"),
            },
        },
        traits: [recoverable({ within: { days: 30 }, by: "manage" })],
    },
    (method) => ({
        create: method.create("manage", {
            fields: ["packageId"],
            isPredicted: false,
        }),
    }),
);
/** A persisted orphan. */
export type Orphan = Select<typeof orphan.table>;
