import { schema } from "@destack/schema";

/** The prefix of each party's wake-up in an alarm clock's storage. */
const WAKEUP_PREFIX = "wakeup:";

/** A durable wake-up the host sets for a loop, such as a Durable Object's alarm. */
export interface Alarm {
    /** Wake the instance at a time, in UTC epoch milliseconds, replacing the earlier wake-up. */
    readonly setAlarm: (at: number) => Promise<void>;
    /** Wake the instance at no time. */
    readonly deleteAlarm: () => Promise<void>;
}

/** Where an alarm clock keeps its parties' wake-ups: a Durable Object's storage, or memory. */
export interface AlarmStorage {
    /** Store a party's wake-up. */
    readonly put: (key: string, value: number) => Promise<void>;
    /** Forget a party's wake-up. */
    readonly delete: (key: string) => Promise<boolean>;
    /** List the wake-ups below a prefix. */
    readonly list: (options: { readonly prefix: string }) => Promise<Map<string, unknown>>;
}

/** The storages of alarm clocks. */
export const AlarmStorage = {
    /** Keep wake-ups in memory, for a host whose parties set them again after a restart. */
    memory(): AlarmStorage {
        const values = new Map<string, number>();

        return {
            put: async (key, value) => {
                values.set(key, value);
            },
            delete: async (key) => values.delete(key),
            list: async ({ prefix }) =>
                new Map([...values].filter(([key]) => key.startsWith(prefix))),
        };
    },
};

/** A clock placing several parties' alarms on one host wake-up. */
export class AlarmClock {
    /** The host's wake-up. */
    readonly #alarm: Alarm;
    /** The parties' wake-ups. */
    readonly #storage: AlarmStorage;
    /** The time the host's wake-up is set to, null for none, unknown after a ring or a restart. */
    #armed: number | null | undefined;

    /** Place parties' alarms on a host's wake-up, their wake-ups in a storage. */
    constructor(alarm: Alarm, storage: AlarmStorage) {
        this.#alarm = alarm;
        this.#storage = storage;
    }

    /** Hand out a party's alarm. */
    alarm(party: string): Alarm {
        return {
            setAlarm: async (at) => {
                await this.#storage.put(`${WAKEUP_PREFIX}${party}`, at);
                await this.#arm();
            },
            deleteAlarm: async () => {
                await this.#storage.delete(`${WAKEUP_PREFIX}${party}`);
                await this.#arm();
            },
        };
    }

    /** Wake the parties due at the host's ring, then set its wake-up at the earliest left. */
    async fire(now: number, wake: (parties: readonly string[]) => Promise<void>): Promise<void> {
        // hand the due parties to the host, which may run more, such as every loop after a restart
        const due = [...(await this.#wakeups())].filter(([, at]) => at <= now);
        await wake(due.map(([party]) => party));

        // forget each woken party's wake-up unless it set another meanwhile
        const after = await this.#wakeups();
        for (const [party, at] of due) {
            if (after.get(party) === at) {
                await this.#storage.delete(`${WAKEUP_PREFIX}${party}`);
            }
        }

        // set the host's wake-up again, as the ring cleared it
        this.#armed = undefined;
        await this.#arm();
    }

    /** Set the host's wake-up at the earliest party's, or clear it at none. */
    async #arm(): Promise<void> {
        // leave an unchanged wake-up
        const times = [...(await this.#wakeups()).values()];
        const earliest = times.length === 0 ? null : Math.min(...times);
        if (earliest === this.#armed) {
            return;
        }

        // move the wake-up, or clear it once no party waits
        this.#armed = earliest;
        await (earliest === null ? this.#alarm.deleteAlarm() : this.#alarm.setAlarm(earliest));
    }

    /** Read each party's wake-up. */
    async #wakeups(): Promise<Map<string, number>> {
        const stored = await this.#storage.list({ prefix: WAKEUP_PREFIX });

        return new Map(
            [...stored].map(([key, at]) => [
                key.slice(WAKEUP_PREFIX.length),
                schema.number().parse(at),
            ]),
        );
    }
}
