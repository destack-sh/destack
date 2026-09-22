import {
    check,
    foreignKey,
    identifier,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { schema } from "@destack/schema";

import { space } from "../space/space.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";

/** Resource records. */
export const resource = table(
    "resource",
    {
        ...recordColumns("resource"),
        ...provenanceColumns(),
        /** The account administering the resource and any explicit host. */
        accountId: identifier("account_id", "account").notNull(),
        /** The space containing the provisioned resource. */
        spaceId: identifier("space_id", "space")
            .notNull()
            .references(() => space.id, {
                onDelete: "restrict",
            }),
        /** The space-local resource name. */
        name: text("name").notNull(),
        /** The immutable package release defining this resource kind. */
        definitionPackageId: identifier("definition_package_id", "package").notNull(),
        /** The defining package's calendar version. */
        definitionVersion: text("definition_version").notNull(),
        /** The definition's exported name in the release manifest. */
        definitionName: text("definition_name").notNull(),
        /** The desired specification validated against the definition schema. */
        spec: json("spec", schema.record(schema.string(), schema.json())).notNull(),
        /** The controller's observed state, validated against the definition schema. */
        status: json("status", schema.record(schema.string(), schema.json()))
            .notNull()
            .default(sql`'{}'`),
        /** The resource kind, such as database or files. */
        kind: text("kind").notNull(),
        /** The requested provider, absent for automatic selection. */
        requestedProviderCode: text("requested_provider"),
        /** The requested provider region within the space's residency. */
        requestedRegionId: identifier("requested_region_id", "region"),
        /** The requested host for host-managed storage. */
        requestedHostId: identifier("requested_host_id", "host"),
        /** The provider supplying the resource, absent before provisioning. */
        providerCode: text("provider"),
        /** The provider-assigned resource reference. */
        reference: text("reference"),
        /** The actual provider region, absent before provisioning. */
        regionId: identifier("region_id", "region"),
        /** The host holding the resource, absent for provider-managed storage. */
        hostId: identifier("host_id", "host"),
        /** Whether explicit resource deletion retains or destroys stored content. */
        retention: text("retention", { enum: ["retain", "delete"] })
            .notNull()
            .default("retain"),

        ...reconciliationColumns(),
    },
    (resource) => [
        ...provenanceChecks("resource", resource),
        check(
            "resource_requested_region",
            sql`${resource.requestedRegionId} IS NULL OR ${resource.requestedProviderCode} IS NOT NULL`,
        ),
        ...reconciliationChecks("resource", resource),
        unique("resource_space_name").on(resource.spaceId, resource.name),
        unique("resource_space_id").on(resource.spaceId, resource.id),
        unique("resource_space_kind").on(resource.spaceId, resource.id, resource.kind),
        foreignKey({
            columns: [resource.accountId, resource.spaceId],
            foreignColumns: [space.accountId, space.id],
        }).onDelete("restrict"),
        check(
            "resource_provider",
            sql`(${resource.providerCode} IS NULL) = (${resource.reference} IS NULL)`,
        ),
        check(
            "resource_region",
            sql`${resource.regionId} IS NULL OR ${resource.providerCode} IS NOT NULL`,
        ),
        check(
            "resource_host",
            sql`${resource.hostId} IS NULL OR ${resource.providerCode} IS NOT NULL`,
        ),
        check("resource_retention", sql`${resource.retention} IN ('retain', 'delete')`),
    ],
);

/** A persisted resource record. */
export type Resource = Select<typeof resource>;
