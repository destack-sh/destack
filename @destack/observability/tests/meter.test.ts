import { expect, onTestFinished, test } from "@destack/test";
import { EventFixture } from "@destack/event/test";
import { defineSku, usage } from "@destack/finance/declare";
import { MeterUsage } from "@destack/finance/server";
import { compute, databaseStorage } from "../src/meter/index.ts";
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
    meter: databaseStorage.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "0.2",
    source: "https://example.com/pricing",
    observedAt: OBSERVED_AT,
});

/** Compute, rated per GB-second. */
const computeSku = defineSku({
    name: "test.compute",
    description: "Memory held while installations run.",
    provider: "test",
    service: "Compute",
    category: "Compute",
    meter: compute.reference,
    pricingUnit: "GB-second",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0000125",
    source: "https://example.com/pricing",
    observedAt: OBSERVED_AT,
});

test("append a space's measured use as usage events once, a failed round's counts in the next, routed to the paying account, which reads 2 GB kept for half a month as 1 GB-month, a level dropping to nothing, and an interval ending with the month in that month", async () => {
    // keep the paying account's routed usage, and fail the first round's routing
    const account = await EventFixture.open("sqlite", [usage]);
    onTestFinished(() => account[Symbol.asyncDispose]());
    let isRouting = false;
    const { events, settle } = await serveObservability({
        skus: [databaseSku, computeSku],
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
    let now = start;
    const rounds: Reading[][] = [
        [
            { meter: databaseStorage, scope: ids.space, value: 2_000_000_000 },
            { meter: compute, scope: ids.space, installation: ids.notes, value: 100 },
        ],
        [
            { meter: databaseStorage, scope: ids.space, value: 0 },
            { meter: compute, scope: ids.space, installation: ids.notes, value: 50 },
        ],
    ];
    const meters = new MeterController(events, async () => rounds.shift() ?? [], {
        skus: [databaseSku, computeSku],
        interval: 1,
        clock: () => now,
    });
    now = (start + end) / 2;
    const failed = await meters.reconcile().then(
        () => "done",
        () => "failed",
    );
    now = end;
    await meters.reconcile();
    await meters.reconcile();
    await settle();

    // keep each event once in the space, and rate the routed copies at the account
    const meterUsage = new MeterUsage(account.store);
    const period = { start, end };
    const kept = await events.query(usage, { scope: ids.space });
    expect({
        failed,
        kept: kept.events
            .toSorted(
                (left, right) =>
                    left.time - right.time || left.keys.meter.localeCompare(right.keys.meter),
            )
            .map((event) => ({
                meter: event.keys.meter,
                installation: event.keys.installation,
                quantity: event.keys.quantity,
                level: event.keys.level,
            })),
        storage: await meterUsage.read(ids.account, databaseStorage, period),
        level: await meterUsage.read(ids.account, databaseStorage, undefined),
        compute: await meterUsage.read(ids.account, compute, period),
    }).toEqual({
        failed: "failed",
        kept: [
            {
                meter: CatalogReference.key(compute.reference),
                installation: ids.notes,
                quantity: 100,
                level: null,
            },
            {
                meter: CatalogReference.key(databaseStorage.reference),
                installation: null,
                quantity: 1_000_000_000,
                level: 2_000_000_000,
            },
            {
                meter: CatalogReference.key(compute.reference),
                installation: ids.notes,
                quantity: 50,
                level: null,
            },
            {
                meter: CatalogReference.key(databaseStorage.reference),
                installation: null,
                quantity: 0,
                level: 0,
            },
        ],
        storage: 1_000_000_000,
        level: 0,
        compute: 150,
    });
});
