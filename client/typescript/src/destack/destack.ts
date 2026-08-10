import { BlobService, blobService } from "../blob/blob.js";
import { daemonService, Daemon } from "../daemon/daemon.js";
import {
    Connection,
    type ConnectionOptions,
} from "../rpc/index.js";
import {
    Workspace,
    workspaceService,
} from "../workspace/workspace.js";

/** One composable connection to a Destack host. */
export class Destack {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Immutable Blob operations. */
    readonly blob: BlobService;
    /** Daemon service. */
    readonly daemon: Daemon;

    /** Bind Destack services to one negotiated connection. */
    constructor(connection: Connection) {
        this.connection = connection;
        this.blob = new BlobService(connection);
        this.daemon = new Daemon(connection);
    }

    /** Connect to one remote Destack daemon. */
    static async connect(
        url: string | URL,
        options: ConnectionOptions = {},
    ): Promise<Destack> {
        const connection = await Connection.connectWebSocket(
            url,
            [blobService, daemonService, workspaceService],
            options,
        );

        return new Destack(connection);
    }

    /** Open one workspace root through this Destack host. */
    openWorkspace(root: string): Promise<Workspace> {
        return this.daemon.openWorkspace(root);
    }

    /** Close this Destack connection. */
    close(): void {
        this.connection.close();
    }
}
