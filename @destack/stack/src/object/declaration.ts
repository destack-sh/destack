import { type ObjectOf, defineObject, field } from "@destack/object";
import { DeclarationName, PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { type Select } from "@destack/db";
import { installation, installationRevision } from "./installation.ts";
import { space } from "@destack/account/object";

/** One declaration a revision's build exports, queried by views, the terminal and agents. */
export const declaration = defineObject({
    name: "declaration",
    plural: "declarations",
    scope: space,
    fields: {
        /** The revision whose build declares it. */
        revision: field.reference(
            (): ObjectOf<{ identity: "installation-revision" }> => installationRevision,
            {
                delete: "cascade",
            },
        ),
        /** The installation of the revision. */
        installation: field.reference((): ObjectOf<{ identity: "installation" }> => installation, {
            delete: "cascade",
        }),
        /** The declaring package. */
        packageId: field.string(PackageId),
        /** The package defining the declaration's kind, whose schema describes it. */
        kindPackageId: field.string(PackageId),
        /** The declaration's kind, such as `view` or `command`. */
        kind: field.string(DeclarationName),
        /** The declaration's name. */
        name: field.string(schema.string().min(1)),
        /** The kind's description, which a reader parses by the kind's schema. */
        description: field.json(schema.json()),
    },
    permissions: ["read", "record"],
    constraints: {
        installationRevision: {
            kind: "foreignKey",
            on: ["installationId", "revisionId"],
            references: { object: () => installationRevision, on: ["installationId", "id"] },
            onDelete: "cascade",
        },
        revisionKindName: { kind: "unique", on: ["revisionId", "kindPackageId", "kind", "name"] },
    },
}).methods((method) => ({
    create: method.create("record", { isInternal: true }),
}));

/** A declaration as its table stores it. */
export type Declaration = Select<typeof declaration.table>;
