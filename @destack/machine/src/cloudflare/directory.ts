import { principal } from "@destack/access";
import type { BlobStorage } from "@destack/resource";
import type { DatabaseConnection } from "@destack/db";
import { implementPackage, ObjectStore } from "@destack/object/server";
import { Package } from "@destack/package";
import { ResourceHandle } from "@destack/resource";
import { defineWorkload } from "@destack/object/workload";
import { type Publisher, Scope } from "@destack/sync";
import machinePackage from "../package.ts";
import type { MachineIdentity } from "@destack/account/client";
import type {} from "@destack/package/import-meta";
import { machine } from "@destack/account/object";
import { Machine } from "../machine/index.ts";

/** What a cloud machine's object follows its replicas with. */
interface DirectoryConfiguration {
    /** The object's database of the replicas. */
    readonly database: DatabaseConnection;
    /** The storage of the replicas' blobs. */
    readonly blobs: BlobStorage;
    /** The machine following them, proven by its key. */
    readonly machine: MachineIdentity;
    /** Stream the account package's replicas as the machine. */
    readonly publisher: Publisher;
}

/** The prefix of a cloud machine's directory blobs in the residency's build bucket. */
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

        // serve them as the machine package's base installation in the machine's scope
        const machineScope = machine.reference(Scope.universe.id, machineId);
        const follows = Machine.follows(machineId);
        const server = await implementPackage(machinePackage, { follows }).base(
            machineScope,
            context.environment.crypto,
        );
        const store = new ObjectStore({
            servers: [server],
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            principal: principal.machine.reference(Scope.universe.id, machineId),
            publisher: () => publisher,
            database,
            blobs,
            environment: context.environment,
            origin: { package: Package.of(machinePackage), machineId },
            machine: machineId,
        });

        return { packages: store.packages };
    },
});
