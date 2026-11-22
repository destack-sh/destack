import { type Follow, type Package, tablesOf, call, type ObjectType } from "@destack/object";
import type { SpaceProvider, StoreProvider } from "@destack/stack/object";
import type { SocketRegistry } from "./socket.ts";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { claim, entitlement, organization } from "@destack/account/object";
import { identityKey } from "@destack/account/identity";
import { spaceReplicated, spaceTables } from "@destack/stack/stack";
import { reference, tag } from "@destack/registry/object";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { S3Credentials } from "@destack/bucket/s3";
import { defineDatabase, type Table } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import type { EventKind } from "@destack/event";
import type { EventStore } from "@destack/event/store";
import type { BlobStorage, Connector } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { BuildStore } from "@destack/build/store";
import type { ExternalStorage } from "@destack/object/server";
import type { Traffic } from "../gateway/traffic.ts";
import type { Reading } from "../meter/index.ts";

/** The tables the kernel reads replicated in every space. */
const KERNEL_REPLICATED: readonly Table[] = [
    ...spaceReplicated,
    organization.table,
    entitlement.table,
    reference.table,
    tag.table,
    claim.table,
];

/** The storage of a machine's spaces, opened as one volume per space. */
export interface SpaceStorage {
    /** The main database every space stores: stack tables, hosted records and events. */
    readonly main: ReturnType<typeof defineDatabase>;
    /** The tables every space replicates of installable packages, declared by its deploys. */
    readonly replicated: readonly Table[];
    /** The event kinds every space records: its calls and its servers' event types. */
    readonly kinds: readonly EventKind[];
    /** The servers' ephemeral object types, which a space stores in memory with its sockets. */
    readonly ephemeral: readonly ObjectType[];
    /** List the spaces whose storage the host stores. */
    readonly list: () => Promise<readonly Identifier<"space">[]>;
    /** Open a space's volume with its database migrated to the space's tables, until it closes. */
    readonly open: (space: Identifier<"space">) => Promise<Volume>;
    /** Delete a closed space's volume. */
    readonly delete: (space: Identifier<"space">) => Promise<void>;
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
    readonly sockets: SocketRegistry;
    /** The providers of the resources the space's installations bind, such as databases. */
    readonly providers: readonly (SpaceProvider | StoreProvider)[];
    /** The connectors of the host's backends the space's installations bind, such as Git. */
    readonly connectors: readonly Connector[];
    /** Open the space's buckets once, serving them over S3 to the space's credentials. */
    readonly buckets: (
        credentials: (scope: string) => Promise<S3Credentials | undefined>,
    ) => CatalogueBucketHost;
    /** Open the graphs of the space's builds once, as external objects. */
    readonly graphs: (
        builds: Pick<
            BuildStore,
            "storeOf" | "contains" | "contents" | "image" | "attachment" | "graph" | "file"
        >,
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

/** The storage of a machine's spaces: the kernel's, each installation adding its own. */
export const SpaceStorage = {
    /** Declare what every space stores before its installations: the kernel's tables and events. */
    declare(): Pick<SpaceStorage, "main" | "replicated" | "kinds" | "ephemeral"> {
        return {
            main: SpaceStorage.main(),
            replicated: SpaceStorage.replicas(),
            kinds: [call.eventKind()],
            ephemeral: [],
        };
    },

    /** Declare a space's main database before its installations: kernel tables and replicas. */
    main(
        additions: {
            /** The packages the space serves, owning their tables and their protocols'. */
            readonly packages?: readonly Pick<Package, "objects" | "tables" | "protocols">[];
            /** The shapes the space follows beside the stack's. */
            readonly follows?: readonly Follow[];
            /** The space's own further tables. */
            readonly tables?: readonly Table[];
        } = {},
    ): ReturnType<typeof defineDatabase> {
        // replicate the rows the kernel reads
        const derived = spaceTables(additions);
        const followed = KERNEL_REPLICATED;

        // own the tables of the packages the space serves and of the protocols they implement
        const served = (additions.packages ?? []).flatMap((each) => [each, ...each.protocols]);
        const owned = new Set(tablesOf({ packages: served, follows: [], tables: [] }).tables);
        const installed = new Set(SpaceStorage.replicas(additions));
        const replicated = new Set<Table>(followed.filter((table) => !owned.has(table)));

        // store the stack's and the additions' other tables, the space's identity keys and the calls
        const tables = [...derived.tables, ...owned, identityKey, ...call.tables].filter(
            (table) => !replicated.has(table) && (owned.has(table) || !installed.has(table)),
        );

        return defineDatabase({
            name: MAIN_DATABASE,
            tables: [...new Set(tables)],
            replicated: [...replicated],
        });
    },

    /** List the tables a space replicates of installable packages, declared by its deploys. */
    replicas(
        additions: {
            /** The shapes the space follows beside the stack's. */
            readonly follows?: readonly Follow[];
        } = {},
    ): readonly Table[] {
        // leave the tables the kernel replicates itself to the storage
        const derived = spaceTables({ follows: additions.follows ?? [] });
        const kernel = new Set<Table>(KERNEL_REPLICATED);
        const replicated = derived.replicated ?? [];

        return [...new Set(replicated)].filter((table) => !kernel.has(table));
    },
};
