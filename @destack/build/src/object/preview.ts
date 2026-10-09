import * as accountObject from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { PackagePath } from "@destack/package/file";
import { schema } from "@destack/schema";
import { BuildView } from "@destack/space/object";
import { checkout } from "./checkout.ts";

/** A checkout's package running live against a space: its build watched and workloads run. */
export const preview = defineObject({
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
        /** The login of the person the preview acts for, on the machine's native client. */
        login: field.string(schema.identifier("login")),

        // observed
        /** Whether the preview starts, runs, stops, stopped or failed. */
        status: field
            .enum(["starting", "running", "stopping", "stopped", "failed"])
            .default("starting"),
        /** The views the current build's outputs mount, keyed by name. */
        views: field.json(schema.record(DeclarationName, BuildView)).default({}),
    },
    permissions: ["read", "start", "stop"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("start", {
            fields: ["checkoutId", "package", "space", "login"],
            isPredicted: false,
        }),
        delete: method.delete("stop"),
        refresh: method.update("control", { isInternal: true, fields: [] }),
    }),
});
/** A preview as its table stores it. */
export type Preview = Select<typeof preview.table>;
