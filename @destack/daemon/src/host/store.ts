import { hostname } from "node:os";
import { v7 } from "uuid";
import { eq, type DatabaseConnection } from "@destack/db";
import { Host } from "../service/index.ts";
import * as stack from "../stack/index.ts";
import type { AuditRecorder } from "@destack/audit";
import { renameHost } from "../audit/action.ts";

/** Public host fields, excluding the singleton storage key. */
const HOST_COLUMNS = {
    deviceId: stack.host.deviceId,
    hostId: stack.host.hostId,
    name: stack.host.name,
    createdAt: stack.host.createdAt,
};

/** Persist this host's identity. */
export class HostStore {
    /** Database containing local identity records. */
    readonly database: DatabaseConnection;

    /** Select the database used by host administration. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Initialize identities under the daemon's exclusive startup lock. */
    async initialize(): Promise<void> {
        // create the host once, independently of enrollment
        const existing = await this.database.select().from(stack.host).get();
        if (!existing) {
            const host = Host.parse({
                deviceId: null,
                hostId: `host-${v7()}`,
                name: hostname(),
                createdAt: Date.now(),
            });
            await this.database
                .insert(stack.host)
                .values({ id: 1, ...host })
                .run();
        }
    }

    /** Read the persisted host identity. */
    async get() {
        return Host.parse(
            await this.database
                .select(HOST_COLUMNS)
                .from(stack.host)
                .where(eq(stack.host.id, 1))
                .get(),
        );
    }

    /** Commit the host display name and its audit record in one transaction. */
    async rename(name: string, audit: AuditRecorder<DatabaseConnection>) {
        return this.database.transaction(async (transaction) => {
            // retain the committed host fields for both the caller and audit record
            const host = await transaction
                .update(stack.host)
                .set({ name })
                .where(eq(stack.host.id, 1))
                .returning(HOST_COLUMNS)
                .get();
            const result = Host.parse(host);

            // persist the audit result atomically with the changed identity
            await audit.record(transaction, renameHost, {
                targets: { host: { type: "host", id: result.hostId } },
                details: { name },
                outcome: "success",
            });

            return result;
        });
    }
}
