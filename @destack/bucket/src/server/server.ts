import type { AuditDestination } from "@destack/audit/server";
import type { DatabaseConnection } from "@destack/db";
import type { Directory } from "@destack/directory";
import { ObjectServer, Subscriber } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import type { CallKey } from "@destack/service/request";
import type { ServiceImplementation } from "@destack/service/server";
import { space } from "@destack/space/object";
import type { Uplink } from "@destack/sync";
import type { BucketProvider } from "../provider/index.ts";
import { bucketService } from "../service/index.ts";

/** What a cell serves its spaces' buckets with: their database, its providers and the spaces it follows. */
export interface BucketOptions {
    /** The database keeping the buckets, with copies of the spaces they live in. */
    readonly database: DatabaseConnection;
    /** The key sensitive call inputs are fingerprinted under in the journal. */
    readonly callKey: CallKey;
    /** The directory keeping the claims of resource names across kinds. */
    readonly directory: Directory;
    /** The providers keeping the buckets, in preference order. */
    readonly providers: readonly BucketProvider[];
    /** The machine keeping the buckets its providers provision, absent for a region. */
    readonly machine: Identifier<"machine"> | null;
    /** The cell serving the spaces, which with the service's name names the copy of their rows. */
    readonly cell: string;
    /** The space service's uplink, streaming the spaces' rows and their chains and receiving their changes. */
    readonly spaces: Uplink;
    /** The audit history the journal delivers calls to. */
    readonly history?: AuditDestination;
}

/** The bucket service with the object server keeping the buckets. */
export interface BucketImplementation extends ServiceImplementation {
    /** The object server keeping the buckets, home of the space service's copies. */
    readonly objects: ObjectServer;
}

/** Implement the bucket service: serve a cell's buckets and their files, provisioning, applying and retiring them through its providers. */
export function implementBucket(options: BucketOptions): BucketImplementation {
    // serve the buckets of the providers, following the spaces they live in
    const { providers } = options;
    const [first] = providers;
    if (first === undefined) {
        throw new TypeError("a bucket service needs a provider");
    }
    const objects: ObjectServer = new ObjectServer({
        objects: { bucket: first.object },
        policies: [space],
        database: options.database,
        callKey: options.callKey,
        directory: options.directory,
        origin: { package: bucketService.package, service: bucketService.name },
        ...(options.history === undefined ? {} : { history: options.history }),
        subscriber: Subscriber.of(options.spaces, () =>
            objects.source.workloadSubscriptions(`${options.cell}/${bucketService.name}`),
        ),
        provisioned: { providers, machine: options.machine },
    });

    // run the providers' controllers beside the buckets'
    return {
        ...objects.implement(
            bucketService,
            providers.flatMap((provider) => provider.controllers),
        ),
        objects,
    };
}
