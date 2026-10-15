import { none, principal } from "@destack/access";
import { and, eq, isNotNull, isNull, type DatabaseConnection, type Select } from "@destack/db";
import { type ObjectOf, defineObject, field } from "@destack/object";
import { Plan } from "@destack/resource";
import { Digest, type Identifier, schema } from "@destack/schema";
import type { Subject } from "@destack/sync";
import { installation } from "./installation.ts";
import { space } from "@destack/account/object";

/** A space's grant to an installation of another space. */
export const bindingGrant = defineObject({
    name: "binding-grant",
    plural: "bindingGrants",
    scope: space,
    controlled: { approval: true },
    fields: {
        /** The other space whose installation the grant admits. */
        granteeSpaceId: field.string(schema.identifier("space")),
        /** The installation of that space the grant admits. */
        granteeInstallationId: field.string(schema.identifier("installation")),
        /** The installation of this space whose package the grantee calls. */
        installation: field.reference((): ObjectOf<{ identity: "installation" }> => installation, {
            delete: "cascade",
        }),
        /** The methods of the installation's package the grantee calls, as `type.method`. */
        calls: field.json(schema.array(schema.string().min(1))).default([]),
    },
    permissions: {
        read: none(),
        list: none(),
        create: none(),
        delete: none(),
        apply: none(),
        grant: none(),
    },
    constraints: {
        installation: {
            kind: "foreignKey",
            on: ["scope", "installationId"],
            references: { object: () => installation, on: ["scope", "id"] },
            onDelete: "cascade",
        },
        granteeSpace: { kind: "check", condition: { granteeSpaceId: { ne: { field: "scope" } } } },
        grantee: {
            kind: "unique",
            on: ["scope", "granteeSpaceId", "granteeInstallationId", "installationId"],
        },
    },
}).methods((method) => ({
    list: method.list("list"),
    create: method.create("create", {
        fields: ["granteeSpaceId", "granteeInstallationId", "installationId", "calls"],
    }),
    delete: method.delete("delete"),

    /** Approve the plan waiting for approval by its digest. */
    approve: method.mutation({
        permission: "apply",
        input: schema.object({
            /** The digest of the reviewed plan. */
            plan: Digest,
        }),
        output: Plan,
    }),

    /** Keep the role letting the grantee call the installation once its plan is approved. */
    grant: method.mutation({
        permission: "grant",
        isInternal: true,
        output: schema.object({}),
    }),
}));

/** A persisted binding grant. */
export type BindingGrant = Select<typeof bindingGrant.table>;

/** Reads of binding grants that the machines routing calls between spaces share. */
export const BindingGrant = {
    /** Read the principal a grant admits: its grantee installation in the grantee's space. */
    grantee(grant: Pick<BindingGrant, "granteeSpaceId" | "granteeInstallationId">): Subject {
        return principal.installation.reference(grant.granteeSpaceId, grant.granteeInstallationId);
    },

    /** Read the procedures an approved grant lets an installation of another space call. */
    async calls(
        database: DatabaseConnection,
        target: Identifier<"installation">,
        grantee: Pick<BindingGrant, "granteeSpaceId" | "granteeInstallationId">,
    ): Promise<readonly string[] | undefined> {
        const [granted] = await database
            .select({ calls: bindingGrant.table.calls })
            .from(bindingGrant.table)
            .where(
                and(
                    eq(bindingGrant.table.installationId, target),
                    eq(bindingGrant.table.granteeSpaceId, grantee.granteeSpaceId),
                    eq(bindingGrant.table.granteeInstallationId, grantee.granteeInstallationId),
                    isNotNull(bindingGrant.table.approvedPlan),
                    isNull(bindingGrant.table.deletedAt),
                ),
            );

        return granted?.calls;
    },
};
