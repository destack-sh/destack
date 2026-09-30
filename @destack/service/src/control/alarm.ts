/** A durable wake-up the host keeps for a loop, such as a Durable Object's alarm, waking an evicted instance when its earliest key is due. */
export interface Alarm {
    /** Wake the instance at a time, in UTC epoch milliseconds, replacing the earlier wake-up. */
    setAlarm(at: number): Promise<void>;
    /** Wake the instance at no time. */
    deleteAlarm(): Promise<void>;
}

/** A clock keeping several loops' alarms on one host wake-up, which rings at the earliest of them. */
export class AlarmClock {
    /** The host's wake-up. */
    readonly #alarm: Alarm;
    /** The time each alarm rings at. */
    readonly #due = new Map<object, number>();
    /** The time the host's wake-up holds, absent for none. */
    #held?: number;

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

                return this.#hold();
            },
            deleteAlarm: () => {
                this.#due.delete(alarm);

                return this.#hold();
            },
        };
    }

    /** Hold the host's wake-up at the earliest alarm. */
    async #hold(): Promise<void> {
        // leave an unchanged wake-up
        const earliest = this.#due.size === 0 ? undefined : Math.min(...this.#due.values());
        if (earliest === this.#held) {
            return;
        }

        // move the wake-up, or clear it once no alarm is set
        this.#held = earliest;
        await (earliest === undefined ? this.#alarm.deleteAlarm() : this.#alarm.setAlarm(earliest));
    }
}

/** A process's wake-up on a timer, for hosts that keep no durable alarms. */
export class TimerAlarm implements Alarm {
    /** Wake the process. */
    readonly #wake: () => void;
    /** The pending timer, absent for none. */
    #timer: ReturnType<typeof setTimeout> | undefined;

    /** Wake a process through a callback. */
    constructor(wake: () => void) {
        this.#wake = wake;
    }

    /** Wake the process at a time, replacing the earlier wake-up. */
    async setAlarm(at: number): Promise<void> {
        clearTimeout(this.#timer);
        this.#timer = setTimeout(() => this.#wake(), Math.max(0, at - Date.now()));
    }

    /** Wake the process at no time. */
    async deleteAlarm(): Promise<void> {
        clearTimeout(this.#timer);
        this.#timer = undefined;
    }
}
