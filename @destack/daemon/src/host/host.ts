import { Watch } from "@destack/service/watch";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import type { Status } from "../service/index.ts";
import type { HostStore } from "./store.ts";

/** Persistent host administration and current daemon observations. */
export class Host {
    /** Persistent host records. */
    readonly storage: HostStore;
    /** Current status and coalesced change notifications. */
    readonly status: Watch<Status>;

    /** Retain the initialized host and its process status. */
    constructor(storage: HostStore, status: Status) {
        this.storage = storage;
        this.status = new Watch(status);
    }

    /** Commit a host name and publish the resulting status. */
    async rename(name: string, audit: AuditRecorder<DatabaseConnection>) {
        // publish only after the host and audit transaction commits
        const host = await this.storage.rename(name, audit);
        this.status.set({ ...this.status.value, host });

        return host;
    }
}
