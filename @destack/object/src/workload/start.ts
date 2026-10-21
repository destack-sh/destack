import { DeclarationName } from "@destack/package";
import { InstallationName } from "../declare/egress.ts";
import { ResourceBinding } from "@destack/resource";
import { schema } from "@destack/schema";

/** The path below which an installation's origin receives webhook requests. */
export const WEBHOOK_PATH = "/.destack/webhook";

/** The path below which a host passes route requests to the workload's own package. */
export const ROUTE_PATH = "/.destack/route";

/** The header that gives the partition of an installation a call goes to. */
export const PARTITION_HEADER = "Destack-Partition";

/** The header with the original URL of a request a front door forwards. */
export const FORWARDED_URL_HEADER = "Destack-Forwarded-Url";

/** The path at which a runner takes its space store's requests to run its objects' hooks. */
export const HOOK_PATH = "/.destack/hook";

/** The first line a host writes to a runner's input, starting one workload of an installation. */
export const WorkloadStart = schema.object({
    /** The instance, the holder name of its leases. */
    instance: schema.identifier("instance"),
    /** The space the installation serves. */
    space: schema.identifier("space"),
    /** The installation the workload runs. */
    installation: schema.identifier("installation"),
    /** The installation's name in its space. */
    name: InstallationName,
    /** The one partition of the installation's database the instance serves. */
    partition: schema.string().min(1).exactOptional(),
    /** The resources the installation binds, by the package's resource name. */
    bindings: schema.record(DeclarationName, ResourceBinding),
    /** The installation's resolved value of each setting its package declares. */
    settings: schema.record(schema.string().min(1), schema.json()),
    /** The package's capabilities the installation runs with, by name. */
    capabilities: schema.array(schema.string().min(1)),
    /** The secret the host and the runner prove each other's requests with. */
    secret: schema.string().min(1),
    /** The host's egress, below which the workload calls addresses as its installation. */
    egress: schema.url(),
    /** The URL callers reach the installation at, its packages mounted below. */
    url: schema.url(),
    /** The share of traces sampled beside every failed or slow one, from 0 to 1. */
    sampling: schema.number().min(0).max(1),
    /** The installation's root as 32 hexadecimal bytes, derived from its space's identity. */
    root: schema.sensitive(schema.string().regex(/^[0-9a-f]{64}$/u)),
});
/** The first line a host writes to a runner's input. */
export type WorkloadStart = schema.Infer<typeof WorkloadStart>;

/** The first line a runner writes to its output once it serves on its socket. */
export const WorkloadReady = schema.object({});
/** The first line a runner writes to its output. */
export type WorkloadReady = schema.Infer<typeof WorkloadReady>;
