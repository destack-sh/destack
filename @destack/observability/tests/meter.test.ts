import { expect, onTestFinished, test, TestClock } from "@destack/test";
import { EventFixture } from "@destack/event/test";
import { defineSku, usage } from "@destack/finance/declare";
import { databaseBytes, memory } from "../src/meter/index.ts";
import { MeterController, type Reading } from "../src/server/index.ts";
import { ids, serveObservability } from "./fixture/observability.ts";
import { CatalogReference } from "@destack/finance";

/** The day the test's prices were read. */
const OBSERVED_AT = "2026-10-01";

/** Database storage, rated per GB-month. */
const databaseSku = defineSku({
    name: "test.database.storage",
    description: "Database storage, kept on average over the month.",
    provider: "test",
    service: "Database",
    category: "Storage",
    meter: databaseBytes.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "0.2",
    source: "https://example.com/pricing",
    observedAt: OBSERVED_AT,
});

/** Memory, rated per GB-second. */
const memorySku = defineSku({
    name: "test.memory",
    description: "Memory held while installations run.",
    provider: "test",
    service: "Compute",
    category: "Compute",
    meter: memory.reference,
    pricingUnit: "GB-second",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0000125",
    source: "https://example.com/pricing",
    observedAt: OBSERVED_AT,
});

test("append a space's measured use as usage events once, a failed round's counts in the next, and route each to the paying account", async () => {
    // keep the paying account's routed usage, and fail the first round's routing
    const account = await EventFixture.open("sqlite", [usage]);
    onTestFinished(() => account[Symbol.asyncDispose]());
    let isRouting = false;
    const { events, settle } = await serveObservability({
        skus: [databaseSku, memorySku],
        targets: async () => {
            if (!isRouting) {
                isRouting = true;
                throw new TypeError("the directory is unreachable");
            }

            return [ids.account];
        },
        deliver: (kind, copies) => account.store.receive(kind, copies),
    });

    // measure 2 GB and 100 GB-seconds over the first half of October, then nothing kept and 50 GB-seconds up to its end, which October keeps
    const start = Date.UTC(2026, 9, 1);
    const end = Date.UTC(2026, 10, 1);
    const clock = new TestClock(start);
    const rounds: Reading[][] = [
        [
            { meter: databaseBytes, scope: ids.space, value: 2_000_000_000 },
            { meter: memory, scope: ids.space, installation: ids.notes, value: 100 },
        ],
        [
            { meter: databaseBytes, scope: ids.space, value: 0 },
            { meter: memory, scope: ids.space, installation: ids.notes, value: 50 },
        ],
    ];
    const meters = new MeterController(events, async () => rounds.shift() ?? [], {
        skus: [databaseSku, memorySku],
        interval: 1,
        clock,
    });
    clock.set((start + end) / 2);
    const failed = await meters.reconcile().then(
        () => "done",
        () => "failed",
    );
    clock.set(end);
    await meters.reconcile();
    await meters.reconcile();
    await settle();

    // keep each event once in the space, and route a copy of each to the account
    const kept = uses(await events.query(usage, { scope: ids.space }));
    const routed = uses(await account.store.query(usage, { scope: ids.account }));
    const expected = [
        {
            meter: CatalogReference.key(memory.reference),
            installation: ids.notes,
            quantity: 100,
            level: null,
        },
        {
            meter: CatalogReference.key(databaseBytes.reference),
            installation: null,
            quantity: 1_000_000_000,
            level: 2_000_000_000,
        },
        {
            meter: CatalogReference.key(memory.reference),
            installation: ids.notes,
            quantity: 50,
            level: null,
        },
        {
            meter: CatalogReference.key(databaseBytes.reference),
            installation: null,
            quantity: 0,
            level: 0,
        },
    ];
    expect({ failed, kept, routed }).toEqual({
        failed: "failed",
        kept: expected,
        routed: expected,
    });
});

/** Read usage events as the test compares them, in time and meter order. */
function uses(page: {
    readonly events: readonly {
        readonly time: number;
        readonly keys: {
            readonly meter: string;
            readonly installation: string | null;
            readonly quantity: number;
            readonly level: number | null;
        };
    }[];
}) {
    return page.events
        .toSorted(
            (left, right) =>
                left.time - right.time || left.keys.meter.localeCompare(right.keys.meter),
        )
        .map((event) => ({
            meter: event.keys.meter,
            installation: event.keys.installation,
            quantity: event.keys.quantity,
            level: event.keys.level,
        }));
}
