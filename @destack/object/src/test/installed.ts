import { none, principal, self } from "@destack/access";
import { PackageId } from "@destack/package";
import { InstallationName } from "../installation/name.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/object.ts";
import { controlled, propagated } from "../trait/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The installations a test stores: the type representing the installation principal. */
export const installation = defineObject(
    {
        name: "installation",
        plural: "installations",
        title: t`${plural(argument("count"), { one: "Installation", other: "Installations" })}`,
        scope: [principal.space],
        key: "name",
        fields: {
            /** The installed package. */
            packageId: field.string(PackageId),
            /** The space-local address. */
            name: field.string(InstallationName),
        },
        constraints: {
            scopePackage: { kind: "unique", on: ["scope", "packageId"] },
            scopeId: { kind: "unique", on: ["scope", "id"] },
        },
        access: {
            represents: principal.installation,
            // let permissions read the scope an installation lives in
            attributes: { scope: "string" },
            permissions: {
                read: none(),
                list: none(),
                install: none(),
                delete: none(),
                replicate: self(),
                run: self(),
                observe: none(),
            },
            reserved: ["replicate", "run"],
        },
        traits: [
            // let its controller finalize an uninstall
            controlled(),
            // propagate each installation into the scopes inside its scope, read there by permission
            propagated({}),
        ],
    },
    (method) => ({
        list: method.list("list"),
        create: method.create("install", { fields: ["packageId", "name"] }),
        delete: method.delete("delete"),
    }),
);
