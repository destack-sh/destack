import { principal, relation } from "@destack/access";
import { check, foreignKey, sql, unique, type Select } from "@destack/db";
import { defineObject, field, method, type ObjectType } from "@destack/object";
import { PackageId } from "@destack/package";
import { Plan } from "@destack/resource";
import { identifier, schema } from "@destack/schema";
import { Digest } from "@destack/package/file";
import { SpaceResource } from "../declare/resource.ts";
import { installation } from "./installation.ts";
import { space } from "./space.ts";

/** The desired states a resource applied, and the plan waiting to apply them. */
export const ResourceStatus = schema.object({
    /** The applied desired states, with the plan waiting for approval or blocked until applied. */
    state: schema
        .object({
            /** The digest of the bound desired states. */
            digest: schema.string(),
            /** The plan waiting to apply them, absent once applied. */
            plan: Plan.optional(),
        })
        .optional(),
});
/** The desired states a resource applied, and the plan waiting to apply them. */
export type ResourceStatus = schema.Infer<typeof ResourceStatus>;

/** The plan an approval accepts. */
const approval = schema.object({
    /** The digest of the reviewed plan. */
    plan: Digest,
});

/** Accept a plan digest for the resource's controller to apply once it plans the same steps. */
const accept = method({ permission: null, isSystem: true, input: approval }).handle((call) =>
    call.revise({ approvedPlan: approval.parse(call.input).plan }),
);

/** Make an installation the owner of a retained resource it declares again, cancelling its deletion. */
const own = method({
    permission: null,
    isSystem: true,
    input: schema.object({
        /** The installation owning the resource. */
        ownerInstallationId: identifier("installation"),
    }),
}).handle((call) => {
    const { ownerInstallationId } = call.input as { readonly ownerInstallationId: string };

    return call.revise({ ownerInstallationId, deletionRequestedAt: null });
});

/** Infrastructure a space has: databases, buckets and vaults, provisioned by providers. */
export const resource = defineObject({
    name: "resource",
    plural: "resources",
    scope: space,
    controlled: true,
    declarable: { schema: SpaceResource },
    fields: {
        /** The space-local resource name. */
        name: field.string(),
        /** The immutable package release defining this resource kind. */
        definitionPackageId: field.string(PackageId),
        /** The defining package's calendar version. */
        definitionVersion: field.string(),
        /** The definition's exported name in the release manifest. */
        definitionName: field.string(),
        /** The desired specification validated against the definition schema. */
        spec: field.json(schema.record(schema.string(), schema.json())),
        /** The desired states the controller applied, and the plan waiting to apply them. */
        status: field.json(ResourceStatus).default({}),
        /** The resource kind, such as database or files. */
        kind: field.string(),
        /** The requested provider, absent for automatic selection. */
        requestedProviderCode: field.string().optional(),
        /** The requested provider location within the space's residency. */
        requestedLocation: field.string().optional(),
        /** The requested host for host-managed storage. */
        requestedHostId: field.string(identifier("host")).optional(),
        /** Whether a host's provider provisions the resource, or its stack declares its reference. */
        origin: field.enum(["provisioned", "declared"]).default("provisioned"),
        /** The provider supplying the resource, absent before provisioning. */
        providerCode: field.string().optional(),
        /** The provider-assigned resource reference. */
        reference: field.string().optional(),
        /** The actual provider location, absent before provisioning. */
        location: field.string().optional(),
        /** The host keeping the resource, absent for provider-managed storage. */
        hostId: field.string(identifier("host")).optional(),
        /** The digest of the plan an approver accepted, applied once the controller plans it again. */
        approvedPlan: field.string(Digest).optional(),
        /** Whether explicit resource deletion retains or destroys stored content. */
        retention: field.enum(["retain", "delete"]).default("retain"),
        /** The application installation that owns the resource, absent for stack resources and after removal. */
        ownerInstallationId: field
            .reference<"installation">((): ObjectType => installation, { delete: "null" })
            .optional(),
    },
    constraints: (resource) => [
        foreignKey({ columns: [resource.scope], foreignColumns: [space.table.id] }).onDelete(
            "restrict",
        ),
        foreignKey({
            columns: [resource.scope, resource.managerInstallationId],
            foreignColumns: [installation.table.scope, installation.table.id],
        }),
        check(
            "resource_requested_location",
            sql`${resource.requestedLocation} IS NULL OR (${resource.requestedProviderCode} IS NOT NULL AND length(${resource.requestedLocation}) > 0)`,
        ),
        unique("resource_scope_name").on(resource.scope, resource.name),
        unique("resource_scope_id").on(resource.scope, resource.id),
        unique("resource_scope_kind").on(resource.scope, resource.id, resource.kind),
        check(
            "resource_provider",
            sql`(${resource.providerCode} IS NULL) = (${resource.reference} IS NULL)`,
        ),
        check(
            "resource_location",
            sql`${resource.location} IS NULL OR (${resource.providerCode} IS NOT NULL AND length(${resource.location}) > 0)`,
        ),
        check(
            "resource_host",
            sql`${resource.hostId} IS NULL OR ${resource.providerCode} IS NOT NULL`,
        ),
    ],
    relations: {
        /** Installations whose live deployments captured the resource. */
        reader: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: { read: relation("reader") },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, {
            isSystem: true,
            fields: [
                "name",
                "kind",
                "definitionPackageId",
                "definitionVersion",
                "definitionName",
                "spec",
                "requestedProviderCode",
                "requestedLocation",
                "requestedHostId",
                "retention",
                "ownerInstallationId",
            ],
        }),
        update: method.update(null, {
            isSystem: true,
            fields: ["definitionVersion", "spec", "retention"],
        }),
        delete: method.delete(null, { isSystem: true }),
        accept,
        own,
    },
});

/** A persisted resource record. */
export type Resource = Select<typeof resource.table>;
