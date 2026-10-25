import { space } from "@destack/account/object";
import { defineObject, field, PackageHandle } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";

/** The documents application, whose people's calls make the space's history. */
export const DOCUMENTS = Package.parse({
    id: "package-019f5530-8000-7000-8000-0000000000d1",
    name: "@example/documents",
    version: "2026.10.0",
});

/** A document people create, update and delete. */
export const document = defineObject(
    {
        name: "document",
        plural: "documents",
        scope: space,
        fields: {
            /** The document's name. */
            name: field.string(schema.string().min(1).max(200)),
        },
        permissions: ["read", "write", "manage"],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("write", { fields: ["name"] }),
        delete: method.delete("manage"),
    }),
    { package: DOCUMENTS },
);

/** The documents package's handle. */
export const documentsPackage = PackageHandle.of(DOCUMENTS, { objects: { document } });

/** The documents package's server. */
export const documentsServer = implementPackage(documentsPackage, { objects: [document] });
