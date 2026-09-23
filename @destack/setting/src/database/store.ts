import type { DatabaseConnection } from "@destack/db";
import { IdempotencyStore } from "@destack/service/database";
import { Watch } from "@destack/service/watch";
import type { AuditRecorder } from "@destack/audit";
import type { Subject } from "@destack/access";
import type { RecordProvenance } from "@destack/model/source";
import { settingRequest } from "../stack/index.ts";
import type { SettingReference } from "../setting/index.ts";

/** Interval for rereading current values changed by another service process. */
const SETTING_POLL_INTERVAL_MS = 1000;

/** Persist conditional settings edits and transactionally record their audit history. */
export class SettingStore {
    /** Database containing the settings schema and audit outbox. */
    readonly database: DatabaseConnection;
    /** Active subscriptions indexed by their selected declaration identities. */
    readonly #subscribers = new Map<Watch<number>, ReadonlySet<string>>();
    /** Database observation timer retained only while subscribed. */
    #timer?: ReturnType<typeof setInterval>;
    /** Shared request retry protocol. */
    readonly requests = new IdempotencyStore(settingRequest);

    /** Retain migrated storage without provisioning a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Invalidate current values after local writes and at bounded polling intervals. */
    async *watch(settings: readonly SettingReference[], signal: AbortSignal): AsyncGenerator<void> {
        // share one polling timer across all resident subscriptions
        const changes = new Watch(0);
        this.#subscribers.set(
            changes,
            new Set(settings.map((setting) => `${setting.packageId}/${setting.name}`)),
        );
        if (this.#subscribers.size === 1) {
            this.#timer = setInterval(() => this.notify(), SETTING_POLL_INTERVAL_MS);
        }
        try {
            for await (const _value of changes.watch(signal)) {
                yield;
            }
        } finally {
            this.#subscribers.delete(changes);
            if (this.#subscribers.size === 0) {
                clearInterval(this.#timer);
            }
        }
    }

    /** Notify resident readers immediately after a successful commit. */
    notify(settings?: readonly SettingReference[]): void {
        const identities = settings?.map((setting) => `${setting.packageId}/${setting.name}`);
        for (const [changes, selected] of this.#subscribers) {
            if (!identities || identities.some((identity) => selected.has(identity))) {
                changes.set(changes.value + 1);
            }
        }
    }
}

/** Verified subject and transactional audit attribution for one mutation. */
export interface SettingWrite {
    /** The represented person checked by the receiving service. */
    readonly subject: Subject;
    /** Recorder carrying the verified caller and receiving scope. */
    readonly audit: AuditRecorder<DatabaseConnection>;
    /** Authorized source revision supplied only by the reconciler. */
    readonly provenance?: RecordProvenance;
}
