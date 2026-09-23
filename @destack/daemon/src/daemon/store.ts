import { join } from "node:path";
import { connect, type Database } from "@destack/db/turso";
import { prepare } from "@destack/db/migration";
import { AuditOutbox } from "@destack/audit/outbox";
import { AuditHistory } from "@destack/audit/history";
import { HostStore } from "../host/store.ts";
import { daemonSchema } from "../stack/index.ts";

/** Domain persistence sharing the local daemon database. */
export class DaemonStore implements AsyncDisposable {
    /** Database containing host records and accepted execution instructions. */
    readonly database: Database;
    /** Persistent host identity. */
    readonly host: HostStore;
    /** Transactional audit delivery. */
    readonly outbox: AuditOutbox;
    /** Searchable local audit history. */
    readonly history: AuditHistory;

    /** Compose domain persistence over one prepared database. */
    private constructor(database: Database) {
        this.database = database;
        this.host = new HostStore(database);
        this.outbox = new AuditOutbox(database);
        this.history = new AuditHistory(database);
    }

    /** Prepare local persistence before starting services. */
    static async open(directory: string): Promise<DaemonStore> {
        // retain the connection so failed initialization can close it
        const database = await connect(join(directory, "daemon.db"), daemonSchema);
        try {
            await prepare(database, [daemonSchema]);
            await database.$client.exec("PRAGMA busy_timeout = 5000;");
            const storage = new DaemonStore(database);
            await storage.host.initialize();

            return storage;
        } catch (error) {
            await database.close();
            throw error;
        }
    }

    /** Close persistence after services and audit delivery stop. */
    async close(): Promise<void> {
        await this.database.close();
    }

    /** Release persistence at the end of its hosting scope. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}
