import { testIdentity } from "@destack/object/test";
import { space } from "@destack/account/object";
import { defineObject, definePackage, field } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { argument, plural, t } from "@destack/locale";

/** The documents application, whose people's calls make the space's history. */
export const DOCUMENTS = PackageVersion.parse({
    id: "package-019f5530-8000-7000-8000-0000000000d1",
    name: "@example/documents",
    version: "2026.10.0",
});

/** A document people create, update and delete. */
export const document = defineObject(
    {
        name: "document",
        plural: "documents",
        title: t`${plural(argument("count"), { one: "Document", other: "Documents" })}`,
        scope: space,
        fields: {
            /** The document's name. */
            name: field.string(schema.string().min(1).max(200)),
        },
        access: {
            permissions: ["read", "write", "manage"],
        },
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("write", { fields: ["name"] }),
        delete: method.delete("manage"),
    }),
    { package: DOCUMENTS },
);

/** The documents package's handle. */
export const documentsPackage = definePackage(
    { ...testIdentity, objects: [document] },
    { package: DOCUMENTS },
);

/** The documents package's server. */
export const documentsServer = implementPackage(documentsPackage, { objects: [document] });
