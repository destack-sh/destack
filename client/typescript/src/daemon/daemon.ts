import {
    DaemonClient,
    daemonService,
} from "../_generated/daemon/daemon.js";
import type { Connection } from "../rpc/index.js";
import { Workspace } from "../workspace/workspace.js";

/** One daemon process over a negotiated RPC connection. */
export class Daemon {
    /** Shared RPC connection. */
    readonly connection: Connection;
    /** Complete generated daemon service client. */
    readonly client: DaemonClient;

    /** Bind the daemon service to one negotiated connection. */
    constructor(connection: Connection) {
        this.connection = connection;
        this.client = new DaemonClient(connection);
    }

    /** Open one root-bound workspace in this daemon. */
    async openWorkspace(root: string): Promise<Workspace> {
        const response = await this.client.openWorkspace({ root });

        return new Workspace(this.connection, response.value.root);
    }

    /** Close one root-bound workspace in this daemon. */
    closeWorkspace(workspace: Workspace) {
        return this.client.closeWorkspace({ root: workspace.root });
    }

    /** Request orderly daemon shutdown. */
    shutdown() {
        return this.client.shutdown(null);
    }
}

export { DaemonClient, daemonService };
