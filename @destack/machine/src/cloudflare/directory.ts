import { principal } from "@destack/access";
import { type BlobStorage, ResourceHandle } from "@destack/resource";
import type { DatabaseConnection, Controller } from "@destack/db/database";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { Package } from "@destack/object";
import { PackageVersion } from "@destack/package";
import { defineWorkload } from "@destack/object/workload";
import { placement } from "@destack/account/object";
import { waitAborted } from "@destack/environment";
import { type Identifier, present } from "@destack/schema";
import { Scope, type Publisher } from "@destack/sync";
import machinePackage from "../package.ts";
import type { MachineKey } from "@destack/account/client";
import type {} from "@destack/package/import-meta";
import { Machine } from "../machine/index.ts";

/** What a cloud machine's object follows its replicas with. */
interface DirectoryConfiguration {
    /** The object's database of the replicas. */
    readonly database: DatabaseConnection;
    /** The storage of the replicas' blobs. */
    readonly blobs: BlobStorage;
    /** The machine following them, proven by its key. */
    readonly machine: MachineKey;
    /** Stream the account package's replicas as the machine. */
    readonly publisher: Publisher;
    /** Start a space's object, which starts only once a request reaches it. */
    readonly start: (space: Identifier<"space">) => Promise<void>;
}

/** The prefix of a cloud machine's directory blobs in the region's bucket. */
export const DIRECTORY_PREFIX = "directory/";

/** The configuration a cloud machine's object binds. */
export const directoryConfiguration = new ResourceHandle<DirectoryConfiguration>(
    import.meta.destack.package,
    "directory",
);

/** The replicas of the directory and of a cloud machine's placements, followed as the machine. */
export const directoryWorkload = defineWorkload({
    name: "directory",
    start: async (context) => {
        // follow the directory, the identities and this machine's placements from the account package
        const configuration = directoryConfiguration.get(context.resources);
        const { database, publisher, blobs } = configuration;
        const { machineId } = configuration.machine;

        // serve the machine package's follows without its types, which only computers serve
        const handle = Package.of(PackageVersion.of(machinePackage), { objects: [] });
        const follows = Machine.follows(machineId);
        const server = implementPackage(handle, { follows });
        const store = new ObjectStore({
            servers: [server],
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            principal: principal.machine.reference(Scope.universe.id, machineId),
            publisher: () => publisher,
            database,
            blobs,
            environment: context.environment,
            origin: { package: PackageVersion.of(machinePackage), machineId },
            machine: machineId,
        });

        // start each space placed or moving here, following it until the space reports it serves
        const starting: Controller<typeof placement.table> = {
            name: "destack.space.start",
            table: placement.table,
            when: { OR: [{ machineId, status: "starting" }, { targetId: machineId }] },
            // start every placed space at once, each followed until it serves, as a kubelet starts pods
            concurrency: Infinity,
            follow: async (rows, following) => {
                // start the space, then wait while its placement stays pending
                const [row] = rows;
                await configuration.start(present(row, "a pending placement").spaceId);
                following.synced();
                await waitAborted(following.signal);
            },
        };
        const control = () => [{ controllers: [starting], projections: [], cursors: [] }];

        return { packages: store.packages, store, control };
    },
});
