import type { Connection } from "../rpc/index.js";
import { BlobStore } from "../blob/blob.js";

import {
    DaemonClient,
    daemonService,
} from "../_generated/daemon/daemon.js";
import type { CreateWorldRequest } from "../_generated/daemon/service/world.js";
import type { WorldId } from "../_generated/runtime/service/world.js";
import type { Snapshot } from "../_generated/runtime/world/world/snapshot.js";

import { Workspace } from "../workspace/workspace.js";

import { World } from "../world/world.js";

/** One daemon process over a negotiated RPC connection. */
export class Daemon {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Complete generated daemon service client. */
    readonly client: DaemonClient;

    /** Shared immutable Blob operations. */
    readonly blobs: BlobStore;

    /** Bind the daemon service to one negotiated connection. */
    constructor(connection: Connection) {
        this.connection = connection;
        this.client = new DaemonClient(connection);

        this.blobs = new BlobStore(connection);
    }

    // =============================================================================
    // Workspace
    // =============================================================================

    /** Open one root-bound workspace in this daemon. */
    async openWorkspace(root: string): Promise<Workspace> {
        const response = await this.client.openWorkspace({ root });

        return new Workspace(this.connection, response.value.root);
    }

    /** Release one root-bound workspace from this connection. */
    closeWorkspace(workspace: Workspace) {
        return this.client.closeWorkspace({ root: workspace.root });
    }

    // =============================================================================
    // World
    // =============================================================================

    /** Create one World in this daemon. */
    async createWorld(request: CreateWorldRequest = {}): Promise<World> {
        const response = await this.client.createWorld(request);

        return this.world(response.value);
    }

    /** Bind one known World identity to this daemon connection. */
    world(worldId: WorldId): World {
        return new World(this, worldId);
    }

    /** Return the Worlds hosted by this daemon. */
    async worlds(): Promise<World[]> {
        const response = await this.client.listWorlds(null);

        return response.value.map((worldId) => this.world(worldId));
    }

    /** Restore one World from a Blob-backed Snapshot. */
    async restoreWorld(snapshot: Snapshot): Promise<World> {
        const response = await this.client.restoreWorld({ snapshot });

        return this.world(response.value);
    }

    /** Close one World in this daemon. */
    async closeWorld(world: World): Promise<void> {
        await this.client.closeWorld({ worldId: world.id });
    }

    // =============================================================================
    // Process
    // =============================================================================

    /** Request orderly daemon shutdown. */
    shutdown() {
        return this.client.shutdown(null);
    }
}

export { DaemonClient, daemonService };
export type { CreateWorldRequest };
