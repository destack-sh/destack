import { type Follow, type Package } from "@destack/object";
import { tablesOf } from "@destack/object";
import type { SpaceProvider, StoreProvider } from "@destack/stack/object";
import type { SocketRegistry } from "./socket.ts";
import { MAIN_DATABASE } from "@destack/stack/declare";
import { claim, entitlement, organization } from "@destack/account/object";
import { identityKey } from "@destack/account/identity";
import { call, type ObjectType } from "@destack/object";
import { spaceReplicated, spaceTables } from "@destack/stack/stack";
import { Forge } from "@destack/registry/client";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import type { S3Credentials } from "@destack/bucket/s3";
import { type DatabaseConnection, defineDatabase, type Table } from "@destack/db";
import type { EventKind, EventStore } from "@destack/event";
import type { BlobStorage, Connector } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { BuildStore } from "@destack/build/store";
import type { ExternalStorage } from "@destack/object/server";
import type { Traffic } from "../gateway/traffic.ts";
import type { Reading } from "../meter/index.ts";

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
            "storeOf" | "contains" | "contents" | "lock" | "referrer" | "file"
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
    declare(): Pick<SpaceStorage, "main" | "kinds" | "ephemeral"> {
        return { main: SpaceStorage.main(), kinds: [call.eventKind()], ephemeral: [] };
    },

    /** Declare a space's main database: the stack's tables, its replicas, and additions. */
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
        // replicate the stack's rows, the account's entitlements, the registry's references and tags, the names it resolves handles by, and the additions' replicas
        const derived = spaceTables(additions);
        const followed = [
            ...spaceReplicated,
            organization.table,
            entitlement.table,
            ...Forge.replicated.map((type) => type.table),
            claim.table,
            ...(derived.replicated ?? []),
        ];

        // own the tables of the packages the space serves and of the protocols they implement
        const served = (additions.packages ?? []).flatMap((each) => [each, ...each.protocols]);
        const owned = new Set(tablesOf({ packages: served, follows: [], tables: [] }).tables);
        const replicated = new Set<Table>(followed.filter((table) => !owned.has(table)));

        // store the stack's and the additions' other tables, the space's identity keys and the calls
        const tables = [...derived.tables, ...owned, identityKey, ...call.tables].filter(
            (table) => !replicated.has(table),
        );

        return defineDatabase({
            name: MAIN_DATABASE,
            tables: [...new Set(tables)],
            replicated: [...replicated],
        });
    },
};
