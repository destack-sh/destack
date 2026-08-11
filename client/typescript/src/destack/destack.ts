import {
    Connection,
    type ConnectionOptions,
} from "../rpc/index.js";

import { daemonService, Daemon } from "../daemon/daemon.js";

import { type BlobStore, blobService } from "../blob/blob.js";

import { workspaceService } from "../workspace/workspace.js";

import { debuggerService } from "../world/debugger.js";
import { hostService } from "../world/host.js";
import { worldService } from "../world/world.js";

/** One composable connection to a Destack host. */
export class Destack {
    /** Shared RPC connection. */
    readonly connection: Connection;

    /** Daemon service. */
    readonly daemon: Daemon;

    /** Bind Destack services to one negotiated connection. */
    constructor(connection: Connection) {
        this.connection = connection;

        this.daemon = new Daemon(connection);
    }

    /** Immutable Blob operations. */
    get blobs(): BlobStore {
        return this.daemon.blobs;
    }

    /** Connect to one remote Destack daemon. */
    static async connect(
        url: string | URL,
        options: ConnectionOptions = {},
    ): Promise<Destack> {
        const connection = await Connection.connectWebSocket(
            url,
            [
                daemonService,

                blobService,

                workspaceService,

                worldService,
                debuggerService,
                hostService,
            ],
            options,
        );

        return new Destack(connection);
    }

    /** Close this Destack connection. */
    close(): void {
        this.connection.close();
    }
}
