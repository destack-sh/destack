import { DeclarationReference } from "@destack/package/declare";
import { EventTime } from "@destack/event";
import { AccessContext, Caller } from "@destack/access";
import { reading } from "@destack/stack/object";
import type { ObjectStore } from "@destack/object/server";
import { journal } from "@destack/object";
import { type Controller, type Following, TABLE } from "@destack/db";
import { Scope } from "@destack/sync";
import { RequestId } from "@destack/sync";
import { MachineError } from "../error/index.ts";
import type { DigestCrypto, Identifier } from "@destack/schema";
import type { Reading } from "./reading.ts";
import type { Meter } from "@destack/telemetry/meter";

/** How long an active space's use gathers before a reading: a minute, the most a restart loses. */
const READING_MILLISECONDS = 60_000;

/** The tables a reading's append writes: its log and the call journal, which wake no meter. */
const OWN_TABLES: ReadonlySet<string> = new Set([
    reading.table[TABLE].sqlName,
    journal[TABLE].sqlName,
]);

/** What a space's meter needs: the space, what it used, the SKUs rating it and who pays for it. */
export interface MeterOptions {
    /** The space measured. */
    readonly space: Identifier<"space">;
    /** Read what the space used over an interval. */
    readonly measure: (from: number, to: number) => Promise<readonly Reading[]>;
    /** The key of the SKU rating each meter on the space's machine, by the meter's key. */
    readonly skus: ReadonlyMap<string, string>;
    /** Read the account paying for the space now. */
    readonly payer: () => Promise<Identifier<"account">>;
    /** Wait until the space's traffic is counted, or until the signal aborts. */
    readonly traffic: (signal: AbortSignal) => Promise<void>;
}

/** The store a space's meters read and append through. */
type MeterStore = Pick<
    ObjectStore,
    "clock" | "database" | "events" | "mutate" | "principal" | "environment"
>;

/** A space's meter: readings a minute into each burst of its activity, none while it idles. */
export class SpaceMeter {
    /** The space's store. */
    readonly #store: MeterStore;
    /** How the space is measured. */
    readonly #options: MeterOptions;

    /** Record a space's use through its store. */
    constructor(store: MeterStore, options: MeterOptions) {
        this.#store = store;
        this.#options = options;
    }

    /** Build the controller following the space's activity while its scope's row stands. */
    controller(): Controller<typeof Scope.table> {
        return {
            name: "meter",
            table: Scope.table,
            pending: { scope: this.#options.space },
            follow: (_rows, following) => this.follow(following),
        };
    }

    /** Read the space's use a minute into each burst of its activity, and once more as it stops. */
    async follow({ signal, synced }: Following): Promise<void> {
        // wait for activity after the commits of the latest reading
        synced();
        let generation = this.#store.database.state.commits.generation;
        while (await this.#active(generation, signal)) {
            // gather a minute of it, reading at once when the space stops meanwhile
            await this.#store.clock.sleep(READING_MILLISECONDS, signal).catch((error: unknown) => {
                if (!signal.aborted) {
                    throw error;
                }
            });
            generation = await this.read();
        }
    }

    /** Measure since the latest reading and append the space's use, answering the generation. */
    async read(): Promise<number> {
        // measure from the latest reading, else from now
        const now = this.#store.clock.now();
        const from = (await this.#latest()) ?? now;
        const measured = await this.#options.measure(from, now);

        // append each reading at its SKU and payer, a level as constant over each month of the interval
        const { space, skus } = this.#options;
        if (measured.length > 0) {
            const payer = await this.#options.payer();
            const items = await Promise.all(
                measured.flatMap((each) =>
                    SpaceMeter.#readingsOf(
                        each,
                        from,
                        now,
                        SpaceMeter.#skuOf(skus, each.meter),
                        payer,
                        this.#store.environment.crypto,
                    ),
                ),
            );
            await this.#store.mutate(AccessContext.of(Caller.of(this.#store.principal), now), {
                id: RequestId.create(this.#store.environment),
                calls: [reading.call("append", { ...reading.in(space), items })],
            });
        }

        return this.#store.database.state.commits.generation;
    }

    /** Wait for the space's traffic or another write past a generation, false once aborted. */
    async #active(generation: number, signal: AbortSignal): Promise<boolean> {
        // race the traffic and the commits that write tables other than the meter's own appends
        const settled = new AbortController();
        const waiting = AbortSignal.any([signal, settled.signal]);
        try {
            await Promise.race([
                this.#options.traffic(waiting),
                this.#store.database.log.waitUntil(
                    async (tables) => [...tables].some((table) => !OWN_TABLES.has(table)),
                    waiting,
                    generation,
                ),
            ]);
        } finally {
            settled.abort();
        }

        return !signal.aborted;
    }

    /** Read the time of the space's latest reading, absent before its first. */
    async #latest(): Promise<number | undefined> {
        const { events } = await this.#store.events.query(
            reading.eventKind(),
            { scopes: [this.#options.space] },
            { limit: 1, order: "descending" },
        );
        const [latest] = events;

        return latest === undefined ? undefined : EventTime.milliseconds(latest.time);
    }

    /** Write an interval's reading as log items: a level per month it spans, a count once. */
    static #readingsOf(
        measured: Reading,
        from: number,
        to: number,
        sku: string,
        payer: string,
        crypto: DigestCrypto,
    ) {
        // record an averaged meter's level beside its level times each month's part of the interval
        const key = DeclarationReference.key(measured.meter.reference);
        const isAverage = measured.meter.definition.aggregation === "average";
        const installation = measured.installation ?? null;
        const parts = isAverage ? SpaceMeter.#months(from, to) : [[from, to] as const];

        return parts.map(async ([start, end]) => ({
            id: await reading.derive(
                measured.scope,
                JSON.stringify([key, installation, end]),
                crypto,
            ),
            time: EventTime.of(end),
            meter: key,
            installationId: installation,
            sku,
            payerId: payer,
            value: isAverage ? measured.value * (end - start) : measured.value,
            data: isAverage ? { level: measured.value } : {},
        }));
    }

    /** Split an interval at the starts of the UTC months it spans. */
    static #months(from: number, to: number): (readonly [number, number])[] {
        // cut at each month start inside the interval
        const parts: (readonly [number, number])[] = [];
        let start = from;
        for (let cut = SpaceMeter.#nextMonth(from); cut < to; cut = SpaceMeter.#nextMonth(cut)) {
            parts.push([start, cut]);
            start = cut;
        }
        parts.push([start, to]);

        return parts;
    }

    /** Read the start of the UTC month after the one a time falls in. */
    static #nextMonth(time: number): number {
        const date = new Date(time);

        return Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1, 1);
    }

    /** Find the SKU rating a meter, refusing a meter the machine rates with none. */
    static #skuOf(skus: ReadonlyMap<string, string>, meter: Meter): string {
        const sku = skus.get(DeclarationReference.key(meter.reference));
        if (sku === undefined) {
            throw MachineError.of(
                "CONFIGURATION_INVALID",
                `the machine rates meter ${meter.name} with no sku`,
            );
        }

        return sku;
    }
}
