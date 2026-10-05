import type { DatabaseConnection } from "@destack/db";
import type { Controller } from "@destack/service/control";
import { SettingValue } from "@destack/setting/object";
import { monitorSegment } from "../segment/table.ts";
import { telemetryRetention } from "../setting/setting.ts";
import { Monitor } from "./monitor.ts";

/** How often each installation's segments are compacted, pruned and swept: every ten minutes. */
const INTERVAL_MILLISECONDS = 10 * 60 * 1000;

/** Compacts, prunes and sweeps the segments of each installation and host scope the catalog names. */
export class SegmentController implements Controller {
    /** The controller's name in reports. */
    readonly name = "segments";
    /** The catalog with commits that list the keys again. */
    readonly watches = [monitorSegment];
    /** The monitor keeping the segments. */
    readonly #monitor: Monitor;
    /** The owner's database with the setting values of its installations. */
    readonly #settings: DatabaseConnection;
    /** The last reconciliation of each key, in Unix milliseconds. */
    readonly #reconciledAt = new Map<string, number>();

    /** Keep a monitor's segments, retained as an owner's settings place them. */
    constructor(monitor: Monitor, settings: DatabaseConnection) {
        this.#monitor = monitor;
        this.#settings = settings;
    }

    /** List each installation and scope host the catalog has segments of. */
    list(): Promise<readonly string[]> {
        return this.#monitor.emitters();
    }

    /** Compact, prune and sweep a key's segments at most once an interval. */
    async reconcile(key: string): Promise<number> {
        // wait out the rest of the interval since the last pass
        const now = Date.now();
        const since = now - (this.#reconciledAt.get(key) ?? 0);
        if (since < INTERVAL_MILLISECONDS) {
            return INTERVAL_MILLISECONDS - since;
        }
        this.#reconciledAt.set(key, now);

        // read the installation's retention setting or the host default
        const { scope, installation } = Monitor.emitter(key);
        const days =
            installation === undefined
                ? telemetryRetention.definition.default
                : await SettingValue.resolve(this.#settings, telemetryRetention, {
                      scope,
                      installation,
                  });

        // merge finished hours
        await this.#monitor.compact(scope, installation, now);

        // drop expired segments
        await this.#monitor.prune(scope, installation, days, now);

        // delete unnamed files
        await this.#monitor.sweep(scope, installation, now);

        return INTERVAL_MILLISECONDS;
    }
}
