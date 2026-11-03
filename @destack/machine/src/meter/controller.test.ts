import { DeclarationReference } from "@destack/package/declare";
import { Event, EventTime } from "@destack/event";
import { reading } from "@destack/stack/object";
import { databaseBytes, memory } from "@destack/telemetry/meter";
import { Identifier, Text } from "@destack/schema";
import { TestSpace } from "@destack/stack/test";
import { expect, test } from "@destack/test";
import { Traffic } from "../gateway/traffic.ts";
import { MeterController } from "./controller.ts";
import type { Reading } from "./reading.ts";
import { METER_OPTIONS, METER_START } from "../computer/test/templates.ts";

/** Order readings by time, then by meter, as the log orders one time's readings by id. */
function byTime(left: Logged, right: Logged): number {
    return (
        left.time - right.time || Text.compare(String(left.keys.meter), String(right.keys.meter))
    );
}

/** A logged reading as the test compares it. */
interface Logged {
    /** When the reading's interval ended. */
    readonly time: number;
    /** The reading's keys, with its meter. */
    readonly keys: { readonly meter: unknown };
}

/** The milliseconds in a minute, the space's reading interval. */
const MINUTE = 60_000;

/** Wait for traffic that never comes, until the signal aborts. */
function quiet(signal: AbortSignal): Promise<void> {
    const { promise, resolve } = Promise.withResolvers<void>();
    signal.addEventListener("abort", () => resolve(), { once: true });

    return promise;
}

/** The start of the month after the scenario's, which an idle space crosses. */
const FEBRUARY_START = Date.UTC(2026, 1, 1);

/** The SKU rating database storage per GB-month, by declaration key. */
const DATABASE_SKU = "package-019f5530-8000-7000-8000-000000000402/test.database.storage";

/** The SKU rating memory per GB-second, by declaration key. */
const MEMORY_SKU = "package-019f5530-8000-7000-8000-000000000402/test.memory";

/** The SKU rating each meter on the test's machine, by the meter's key. */
const SKUS = new Map([
    [DeclarationReference.key(databaseBytes.reference), DATABASE_SKU],
    [DeclarationReference.key(memory.reference), MEMORY_SKU],
]);

test("append a space's use as readings each round, measuring a failed round in the next and resuming after a restart from the latest reading", async () => {
    // serve a space, in a world the scenario moves
    const fixture = await TestSpace.open(METER_OPTIONS);
    const world = fixture.environment;
    const served = await fixture.serve({ isControlled: false });
    const { store } = served;
    const { events } = store;
    const spaceId = fixture.id;
    const accountId = fixture.account;
    const notes = Identifier.create("installation", world);

    // measure, fail once, measure again, then measure after a restart
    const rounds: (readonly Reading[] | Error)[] = [
        [
            { meter: databaseBytes, scope: spaceId, value: 2_000_000_000 },
            { meter: memory, scope: spaceId, installation: notes, value: 100 },
        ],
        [],
        new Error("the machine's records are unreachable"),
        [{ meter: memory, scope: spaceId, installation: notes, value: 50 }],
        [{ meter: databaseBytes, scope: spaceId, value: 1_000_000_000 }],
    ];
    const measured: [number, number][] = [];
    const options = {
        space: spaceId,
        skus: SKUS,
        payer: async () => accountId,
        traffic: quiet,
        measure: async (from: number, to: number) => {
            measured.push([from, to]);
            const round = rounds.shift() ?? [];
            if (round instanceof Error) {
                throw round;
            }

            return round;
        },
    };

    // read twice, fail, read from the latest reading, then after a restart
    const meters = new MeterController(store, options);
    await meters.read();
    await fixture.simulation.run(METER_START + 2 * MINUTE);
    await meters.read();
    await fixture.simulation.run(METER_START + 3 * MINUTE);
    const failed = await meters.read().then(
        () => "measured",
        (error: unknown) => (error instanceof Error ? error.message : String(error)),
    );
    await fixture.simulation.run(METER_START + 4 * MINUTE);
    await meters.read();
    await fixture.simulation.run(METER_START + 5 * MINUTE);
    await new MeterController(store, options).read();

    // record each reading once, a level as constant over its interval
    const read = async (
        meter: typeof memory,
        sku: string,
        installation: string | null,
        to: number,
        value: number,
        level: number | null,
    ) => {
        const key = DeclarationReference.key(meter.reference);

        return {
            id: await Event.identity(spaceId, [key, installation, to], world.crypto),
            time: EventTime.of(to),
            keys: {
                meter: key,
                installationId: installation,
                sku,
                payerId: accountId,
                value,
            },
            data: level === null ? {} : { level },
        };
    };
    const { events: logged } = await events.query(
        reading.eventKind(),
        { scopes: [spaceId] },
        { order: "ascending" },
    );
    expect({
        measured,
        failed,
        logged: logged
            .map(({ id, time, keys, data }) => ({ id, time, keys, data }))
            .toSorted(byTime),
    }).toEqual({
        measured: [
            [METER_START, METER_START],
            [METER_START, METER_START + 2 * MINUTE],
            [METER_START, METER_START + 3 * MINUTE],
            [METER_START, METER_START + 4 * MINUTE],
            [METER_START + 4 * MINUTE, METER_START + 5 * MINUTE],
        ],
        failed: "the machine's records are unreachable",
        logged: [
            await read(databaseBytes, DATABASE_SKU, null, METER_START, 0, 2e9),
            await read(memory, MEMORY_SKU, notes, METER_START, 100, null),
            await read(memory, MEMORY_SKU, notes, METER_START + 4 * MINUTE, 50, null),
            await read(
                databaseBytes,
                DATABASE_SKU,
                null,
                METER_START + 5 * MINUTE,
                1e9 * MINUTE,
                1e9,
            ),
        ].toSorted(byTime),
    });
});

test("read nothing while a space idles, then its use a minute into its next traffic, a level split at the month it crossed", async () => {
    // serve a space, read once, then follow its activity
    const fixture = await TestSpace.open(METER_OPTIONS);
    const world = fixture.environment;
    const served = await fixture.serve({ isControlled: false });
    const { store } = served;
    const spaceId = fixture.id;
    const traffic = new Traffic();
    const measured: [number, number][] = [];
    const meters = new MeterController(store, {
        space: spaceId,
        skus: SKUS,
        payer: async () => fixture.account,
        traffic: (signal) => traffic.next(signal),
        measure: async (from, to) => {
            measured.push([from, to]);

            return [{ meter: databaseBytes, scope: spaceId, value: 1_000_000_000 }];
        },
    });
    await meters.read();
    const stopping = new AbortController();
    const following = meters.reconcile(spaceId, {
        signal: stopping.signal,
        changed: async () => undefined,
        synced: () => undefined,
    });

    // idle into the next month, then serve a request and let a minute pass
    await fixture.simulation.jump(FEBRUARY_START + MINUTE);
    const idle = measured.length;
    traffic.countRequest(spaceId, Identifier.create("installation", world));
    await fixture.simulation.run(FEBRUARY_START + 3 * MINUTE);
    stopping.abort();
    await following;

    // read once more, splitting the level's interval at February's start
    const { events: logged } = await store.events.query(
        reading.eventKind(),
        { scopes: [spaceId] },
        { order: "ascending" },
    );
    expect({
        idle,
        measured,
        logged: logged.map(({ time, keys }) => [EventTime.milliseconds(time), keys.value]),
    }).toEqual({
        idle: 1,
        measured: [
            [METER_START, METER_START],
            [METER_START, FEBRUARY_START + 2 * MINUTE],
        ],
        logged: [
            [METER_START, 0],
            [FEBRUARY_START, 1e9 * (FEBRUARY_START - METER_START)],
            [FEBRUARY_START + 2 * MINUTE, 1e9 * 2 * MINUTE],
        ],
    });
});
