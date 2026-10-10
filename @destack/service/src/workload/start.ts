import { DeclarationName } from "@destack/package";
import { ResourceBinding } from "@destack/resource";
import { defineSchema, schema } from "@destack/schema";

/** The path below which an installation's origin receives webhook requests, and a host forwards them to a runner. */
export const WEBHOOK_PATH = "/.destack/webhook";

/** The path below which a host hands a service the requests a route sends it, by the service's name, and at which the service answers them below its mount. */
export const ROUTE_PATH = "/.destack/route";

/** The header a caller names the partition of an installation its call goes to with, the installation's root without it. */
export const PARTITION_HEADER = "Destack-Partition";

/** The header carrying the URL a request a front door hands on asked for, such as a routed request's. */
export const FORWARDED_URL_HEADER = "Destack-Forwarded-Url";

/** The path on a host's egress origin at which the host takes its instances' journals and relays them to their spaces' histories. */
export const JOURNAL_PATH = "/.destack/journal";

/** The first line a host writes to a runner's input, starting one workload of an installation. */
export const WorkloadStart = defineSchema(
    schema.object({
        /** The instance, the holder name of its leases. */
        instance: schema.identifier("instance"),
        /** The space the installation serves. */
        scope: schema.identifier("space"),
        /** The installation the workload runs. */
        installation: schema.identifier("installation"),
        /** The one partition of the installation's database the instance serves, every partition when absent. */
        partition: schema.string().min(1).exactOptional(),
        /** The resources the installation binds, by the package's resource name. */
        bindings: schema.record(DeclarationName, ResourceBinding),
        /** The installation's resolved value of each setting its package declares, by `<package>/<name>`. */
        settings: schema.record(schema.string().min(1), schema.json()),
        /** The secret the host and the runner prove each other's requests with. */
        secret: schema.string().min(1),
        /** The host's egress, below which the workload reaches addresses as its installation. */
        egress: schema.url(),
        /** The share of traces the workload keeps beside every failed or slow one, from 0 to 1. */
        sampling: schema.number().min(0).max(1),
        /** The installation's journal key, 32 bytes as hexadecimal, which the host derives for it. */
        callKey: schema.sensitive(schema.string().regex(/^[0-9a-f]{64}$/u)),
    }),
);
/** The first line a host writes to a runner's input. */
export type WorkloadStart = schema.Infer<typeof WorkloadStart>;

/** The first line a runner writes to its output once it serves on its socket. */
export const WorkloadReady = defineSchema(schema.object({}));
/** The first line a runner writes to its output. */
export type WorkloadReady = schema.Infer<typeof WorkloadReady>;
