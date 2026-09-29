import { DeclarationName } from "@destack/package";
import { defineSchema, identifier, schema } from "@destack/schema";

/** A provisioned resource a host binds to a starting workload. */
export const ResourceBinding = defineSchema(
    schema.object({
        /** The resource. */
        resource: identifier("resource"),
        /** The resource kind, such as database. */
        kind: schema.string().min(1),
        /** The provider holding the resource, such as sqlite. */
        provider: schema.string().min(1),
        /** The provider's reference, such as a file URL. */
        reference: schema.string().min(1),
        /** The declared specification. */
        spec: schema.record(schema.string(), schema.json()),
    }),
);
/** A provisioned resource a host binds to a starting workload. */
export type ResourceBinding = schema.Infer<typeof ResourceBinding>;

/** The first line a host writes to a runner's input, starting one workload of an installation. */
export const WorkloadStart = defineSchema(
    schema.object({
        /** The instance, the holder name of its leases. */
        instance: identifier("instance"),
        /** The workload, by its name in the output. */
        workload: DeclarationName,
        /** The space the installation serves. */
        scope: identifier("space"),
        /** The installation the workload runs. */
        installation: identifier("installation"),
        /** The resources the installation binds, by the package's resource name. */
        bindings: schema.record(DeclarationName, ResourceBinding),
        /** The installation's short-lived credential. */
        credential: schema.string().min(1),
        /** The secret the host proves the callers it forwards with. */
        secret: schema.string().min(1),
        /** The audit service of the installation's space. */
        audit: schema.url(),
        /** The space service of the installation's holder, relaying the space's access. */
        space: schema.url(),
    }),
);
/** The first line a host writes to a runner's input. */
export type WorkloadStart = schema.Infer<typeof WorkloadStart>;

/** A later line a host writes to a runner's input, renewing the installation's credential. */
export const WorkloadRenewal = defineSchema(
    schema.object({
        /** The installation's next credential. */
        credential: schema.string().min(1),
    }),
);
/** A later line a host writes to a runner's input. */
export type WorkloadRenewal = schema.Infer<typeof WorkloadRenewal>;

/** The first line a runner writes to its output, once it serves. */
export const WorkloadReady = defineSchema(
    schema.object({
        /** The loopback port serving the package's service below its mount. */
        port: schema.number().int().min(1).max(65_535),
    }),
);
/** The first line a runner writes to its output. */
export type WorkloadReady = schema.Infer<typeof WorkloadReady>;
