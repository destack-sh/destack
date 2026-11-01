import { contained, principal, through, union } from "@destack/access";
import * as accountObject from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { schema } from "@destack/schema";
import { ViewDescription } from "@destack/package/manifest";
import { checkout } from "./checkout.ts";

/** A checkout's package running live against a space: its build watched and workloads run. */
export const preview = defineObject(
    {
        name: "preview",
        plural: "previews",
        scope: accountObject.machine,
        controlled: true,
        fields: {
            /** The checkout with the package. */
            checkout: field.reference(checkout, { delete: "cascade" }),
            /** The package directory, relative to the checkout's root. */
            package: field.string(schema.union([schema.literal("."), PackagePath])),
            /** The space supplying resources and permissions. */
            space: field.string(schema.identifier("space")),
            /** The login of the person the preview acts for, on the machine's native device. */
            login: field.string(schema.identifier("login")),

            // observed
            /** The views the current build's outputs mount, keyed by name. */
            views: field.json(schema.record(DeclarationName, ViewDescription)).default({}),
        },
        // let the machine's local client and the people who operate the machine run its previews
        permissions: {
            read: union(contained(principal.machine), through("scope", "operate")),
            start: union(contained(principal.machine), through("scope", "operate")),
            stop: union(contained(principal.machine), through("scope", "operate")),
        },
    },
    (method) => ({
        create: method.create("start", {
            fields: ["checkoutId", "package", "space", "login"],
            isPredicted: false,
        }),
        delete: method.delete("stop"),
        refresh: method.update("control", { visibility: "package", fields: [] }),
    }),
);
/** A preview as its table stores it. */
export type Preview = Select<typeof preview.table>;
