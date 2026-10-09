import { type EventStore, EventTime } from "@destack/event";
import { type Meter, type Sku } from "@destack/finance";
import { usage } from "@destack/finance/declare";
import type { Controller } from "@destack/service/control";
import type { Identifier } from "@destack/schema";
import type { Reading } from "./reading.ts";
import { CatalogReference } from "@destack/finance";

/** How often machines measure their spaces by default: hourly, as usage is billed. */
const INTERVAL_MILLISECONDS = 3_600_000;

/** A space's readings over one measured interval, in Unix milliseconds, waiting to be appended as usage. */
interface Measurement {
    /** The space measured. */
    readonly scope: Identifier<"space">;
    /** Its readings over the interval. */
    readonly readings: readonly Reading[];
    /** The interval's start. */
    readonly from: number;
    /** The interval's end. */
    readonly to: number;
}

/** How a machine measures its spaces: the SKUs rating its meters, and its interval and clock. */
export interface MeterOptions {
    /** The SKUs rating each meter on the machine, one per meter. */
    readonly skus: readonly Sku[];
    /** How often the machine measures, in milliseconds, hourly by default. */
    readonly interval?: number;
    /** Read the current time in Unix milliseconds. */
    readonly clock?: () => number;
}

/** Measures what a machine's spaces use each interval, ending on its multiples within a month, and appends it as usage events stamped at the interval's start, once each, which their route copies to the accounts paying for them: bytes as their share of the month, amounts as measured. */
export class MeterController implements Controller {
    /** The controller's name in reports. */
    readonly name = "meters";
    /** The events the usage is appended to. */
    readonly #events: EventStore;
    /** Read what the machine's spaces use. */
    readonly #measure: (from: number, to: number) => Promise<readonly Reading[]>;
    /** The SKUs rating each meter on the machine. */
    readonly #skus: readonly Sku[];
    /** How often the machine measures, in milliseconds. */
    readonly #interval: number;
    /** Read the current time in Unix milliseconds. */
    readonly #clock: () => number;
    /** The end of the last measured interval, in Unix milliseconds. */
    #measuredAt: number;
    /** The measurements not appended yet, oldest first, kept across a failed append so no count is dropped. */
    #pending: readonly Measurement[] = [];

    /** Measure through a machine's reading from now on, rating each meter by its SKU on the machine. */
    constructor(
        events: EventStore,
        measure: (from: number, to: number) => Promise<readonly Reading[]>,
        options: MeterOptions,
    ) {
        // keep the events, the reading and the SKUs, measuring from now on
        this.#events = events;
        this.#measure = measure;
        this.#skus = options.skus;
        this.#interval = options.interval ?? INTERVAL_MILLISECONDS;
        this.#clock = options.clock ?? Date.now;
        this.#measuredAt = this.#clock();
    }

    /** List the one key the machine measures under. */
    async list(): Promise<readonly string[]> {
        return [this.name];
    }

    /** Measure the time since the last reading up to the last interval boundary within its month, append every measurement not appended yet, then wait for the next. */
    async reconcile(): Promise<number> {
        // wait for the next multiple of the interval, or the month's end before it
        const from = this.#measuredAt;
        const now = this.#clock();
        const monthEnd = nextMonth(from);
        const boundary = Math.min(
            (Math.floor(from / this.#interval) + 1) * this.#interval,
            monthEnd,
        );
        if (now < boundary) {
            return boundary - now;
        }

        // measure up to the last boundary passed in the month, keeping each space's readings until they are appended
        const to = Math.min(Math.floor(now / this.#interval) * this.#interval, monthEnd);
        const readings = await this.#measure(from, to);
        this.#measuredAt = to;
        const measured = [...Map.groupBy(readings, (reading) => reading.scope)].map(
            ([scope, read]): Measurement => ({ scope, readings: read, from, to }),
        );
        this.#pending = [...this.#pending, ...measured];

        // append each measurement oldest first, keeping a failed one and every later one of its space
        const failures = new Map<Identifier<"space">, unknown>();
        const completed = new Set<Measurement>();
        for (const measurement of this.#pending) {
            if (failures.has(measurement.scope)) {
                continue;
            }
            try {
                await this.#events.append(usage, this.#usage(measurement));
                completed.add(measurement);
            } catch (error) {
                failures.set(measurement.scope, error);
            }
        }

        // keep the measurements left to append, and report the failures, which the next round appends again
        this.#pending = this.#pending.filter((measurement) => !completed.has(measurement));
        if (failures.size > 0) {
            throw new AggregateError(
                [...failures.values()],
                "meters could not append some spaces' usage",
            );
        }

        return 0;
    }

    /** Write a measurement as one usage event per meter and installation, named by the interval so a repeated append keeps it once. */
    #usage(measurement: Measurement) {
        // sum the readings of each meter and installation
        const { scope, readings, from, to } = measurement;
        const totals = Map.groupBy(
            readings,
            (reading) =>
                `${CatalogReference.key(reading.meter.reference)}\0${reading.installation ?? ""}`,
        );

        return [...totals.values()].map((summed) => {
            // rate the meter by its SKU on the machine
            const [first] = summed;
            if (first === undefined) {
                throw new TypeError("a meter's total sums no reading");
            }
            const { meter, installation } = first;
            const sku = skuOf(this.#skus, meter);
            const value = summed.reduce((total, reading) => total + reading.value, 0);
            const key = CatalogReference.key(meter.reference);

            // count bytes as their share of the month, amounts as measured
            const isAverage = meter.definition.aggregation === "average";
            const quantity = isAverage ? (value * (to - from)) / monthLength(from) : value;

            return {
                scope,
                id: `${key}:${installation ?? scope}:${String(to)}`,
                time: EventTime.of(from),
                keys: {
                    meter: key,
                    sku: CatalogReference.key(sku.reference),
                    installation: installation ?? null,
                    quantity,
                    level: isAverage ? value : null,
                },
                data: { unit: meter.definition.unit, from, to },
            };
        });
    }
}

/** Find the one SKU rating a meter on the machine, refusing a meter rated by none or several. */
function skuOf(skus: readonly Sku[], meter: Meter): Sku {
    // find the SKUs of the meter, requiring exactly one
    const rating = skus.filter(
        (each) =>
            CatalogReference.key(each.definition.meter) === CatalogReference.key(meter.reference),
    );
    const [sku] = rating;
    if (sku === undefined || rating.length > 1) {
        throw new TypeError(`the machine rates meter ${meter.name} by ${rating.length} skus`);
    }

    return sku;
}

/** Read the start of the UTC month after the one a time falls in, in Unix milliseconds. */
function nextMonth(time: number): number {
    const date = new Date(time);

    return Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1);
}

/** Read the length of the UTC month a time falls in, in milliseconds. */
function monthLength(time: number): number {
    const date = new Date(time);

    return nextMonth(time) - Date.UTC(date.getUTCFullYear(), date.getUTCMonth());
}
