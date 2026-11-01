import { MAIN_DATABASE } from "@destack/stack/declare";
import { claim, entitlement, organization } from "@destack/account/object";
import { identityKey } from "@destack/account/identity";
import { call, type ObjectType } from "@destack/object";
import { spaceReplicated, spaceTables } from "@destack/stack/stack";
import { Forge } from "@destack/forge/client";
import { gitStorage } from "@destack/forge/storage";
import { mail } from "@destack/message/object";
import * as stackObject from "@destack/stack/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { S3Credentials } from "@destack/bucket/s3";
import { type DatabaseConnection, defineDatabase, type Table } from "@destack/db";
import type { EventKind, EventStore } from "@destack/event";
import type { BlobStorage, Connector } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { SpaceBuildStore, SpaceOptions } from "@destack/stack/server";
import type { ExternalStorage, PackageImplementation } from "@destack/object/server";
import type { Traffic } from "../gateway/traffic.ts";
import type { Reading } from "../meter/index.ts";

/** The record types of the kinds every space's storage hosts: databases, Git and mail. */
const HOSTED = [stackObject.database, gitStorage, mail];

/** The storage of a machine's spaces, opened as one volume per space. */
export interface SpaceStorage {
    /** The main database every space stores: stack tables, hosted records and events. */
    readonly main: ReturnType<typeof defineDatabase>;
    /** The event kinds every space records: its calls and its servers' event types. */
    readonly kinds: readonly EventKind[];
    /** The servers' ephemeral object types, which a space stores in memory with its sockets. */
    readonly ephemeral: readonly ObjectType[];
    /** List the spaces whose storage the host stores. */
    readonly list: () => Promise<readonly Identifier<"space">[]>;
    /** Open a space's volume with its database migrated to the space's tables, until it closes. */
    readonly open: (space: Identifier<"space">) => Promise<Volume>;
}

/** One space's volume while it runs: its database, events, blobs, buckets, sockets and graphs. */
export interface Volume {
    /** The space's database. */
    readonly database: DatabaseConnection;
    /** The space's events: its calls, its telemetry and analytics. */
    readonly events: EventStore;
    /** The storage of the space's blobs, which its database's blob columns reference. */
    readonly blobs: BlobStorage;
    /** The registry of the space's live sockets and presence. */
    readonly sockets: SpaceOptions["sockets"];
    /** The providers of the resources the space's installations bind, such as databases. */
    readonly providers: SpaceOptions["providers"];
    /** The connectors of the host's backends the space's installations bind, such as Git. */
    readonly connectors: readonly Connector[];
    /** Open the space's buckets once, serving them over S3 to the space's credentials. */
    readonly buckets: (
        credentials: (scope: string) => Promise<S3Credentials | undefined>,
    ) => CatalogueBucketHost;
    /** Open the graphs of the space's builds once, as external objects. */
    readonly graphs: (
        builds: Pick<SpaceBuildStore, "store" | "contains" | "contents" | "referrer" | "file">,
    ) => ExternalStorage;
    /** Read what the space used over an interval, with the traffic its gateway counted. */
    readonly measure: (
        buckets: Pick<CatalogueBucketHost, "bytes">,
        from: number,
        to: number,
        traffic: Traffic,
    ) => Promise<Reading[]>;
    /** Close what the host opened for the space once it stops. */
    readonly close: () => Promise<void>;
}

/** The storage of a machine's spaces, as its servers declare what every space stores. */
export const SpaceStorage = {
    /** Declare what every space stores, by the servers a machine runs and the kinds it hosts. */
    declare(
        servers: readonly PackageImplementation[],
    ): Pick<SpaceStorage, "main" | "kinds" | "ephemeral"> {
        // read the servers' types by where they are stored
        const types = servers.flatMap((server) => Object.values(server.package.objects));
        const events = [call, ...types.filter((type) => type.storage === "events")];

        // store the stack's tables, the space's identity keys, the hosted records and the events
        const tables = new Set<Table>([
            ...spaceTables({}).tables,
            identityKey,
            ...HOSTED.flatMap((type) => type.tables),
            ...events.flatMap((type) => type.tables),
        ]);

        // replicate the stack's rows, the account's entitlements, the forge's references and tags, and the names it resolves handles by
        return {
            main: defineDatabase({
                name: MAIN_DATABASE,
                tables: [...tables],
                replicated: [
                    ...spaceReplicated,
                    organization.table,
                    entitlement.table,
                    ...Forge.replicated.map((type) => type.table),
                    claim.table,
                ],
            }),
            kinds: events.map((type) => type.eventKind()),
            ephemeral: types.filter((type) => type.storage === "ephemeral"),
        };
    },
};
