import { DeclarationName } from "@destack/package";
import { ResourceBinding } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";

/** The path below which an installation's origin receives webhook requests, and a host forwards them to a runner. */
export const WEBHOOK_PATH = "/.destack/webhook";

/** The first line a host writes to a runner's input, starting one workload of an installation. */
export const WorkloadStart = defineSchema(
    schema.object({
        /** The instance, the holder name of its leases. */
        instance: identifier("instance"),
        /** The space the installation serves. */
        scope: identifier("space"),
        /** The installation the workload runs. */
        installation: identifier("installation"),
        /** The resources the installation binds, by the package's resource name. */
        bindings: schema.record(DeclarationName, ResourceBinding),
        /** The secret the host and the runner prove each other's requests with. */
        secret: schema.string().min(1),
        /** The host's egress, below which the workload reaches addresses as its installation. */
        egress: schema.url(),
        /** The share of traces the workload keeps beside every failed or slow one, from 0 to 1. */
        sampling: schema.number().min(0).max(1),
    }),
);
/** The first line a host writes to a runner's input. */
export type WorkloadStart = schema.Infer<typeof WorkloadStart>;

/** The first line a runner writes to its output after it serves. */
export const WorkloadReady = defineSchema(
    schema.object({
        /** The loopback port serving the package's service below its mount. */
        port: schema.number().int().min(1).max(65_535),
    }),
);
/** The first line a runner writes to its output. */
export type WorkloadReady = schema.Infer<typeof WorkloadReady>;
