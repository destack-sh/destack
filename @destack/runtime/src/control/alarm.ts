/** A durable wake-up the host keeps for a loop, such as a Durable Object's alarm. */
export interface Alarm {
    /** Wake the instance at a time, in UTC epoch milliseconds, replacing the earlier wake-up. */
    readonly setAlarm: (at: number) => Promise<void>;
    /** Wake the instance at no time. */
    readonly deleteAlarm: () => Promise<void>;
}

/** A clock that keeps several loops' alarms on one host wake-up. */
export class AlarmClock {
    /** The host's wake-up. */
    readonly #alarm: Alarm;
    /** The time each alarm rings at. */
    readonly #due = new Map<object, number>();
    /** The time the host's wake-up is set to, absent for none. */
    #current: number | undefined;

    /** Keep alarms on a host's wake-up. */
    constructor(alarm: Alarm) {
        this.#alarm = alarm;
    }

    /** Hand out one loop's alarm. */
    alarm(): Alarm {
        const alarm = {};

        return {
            setAlarm: (at) => {
                this.#due.set(alarm, at);

                return this.#update();
            },
            deleteAlarm: () => {
                this.#due.delete(alarm);

                return this.#update();
            },
        };
    }

    /** Set the host's wake-up again at the earliest alarm. */
    async rang(): Promise<void> {
        this.#current = undefined;
        await this.#update();
    }

    /** Move the host's wake-up to the earliest alarm. */
    async #update(): Promise<void> {
        // leave an unchanged wake-up
        const earliest = this.#due.size === 0 ? undefined : Math.min(...this.#due.values());
        if (earliest === this.#current) {
            return;
        }

        // move the wake-up or clear it once no alarm is set
        this.#current = earliest;
        await (earliest === undefined ? this.#alarm.deleteAlarm() : this.#alarm.setAlarm(earliest));
    }
}
