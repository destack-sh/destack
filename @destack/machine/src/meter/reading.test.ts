import { EventTime } from "@destack/event";
import { usage } from "@destack/stack/object";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { Reading } from "./reading.ts";

/** The start of October 2026, the month the readings fall in. */
const START = Date.UTC(2026, 9, 1);

/** An hour in milliseconds. */
const HOUR = 3_600_000;

/** October's length in milliseconds. */
const OCTOBER = 31 * 86_400_000;

/** A gigabyte. */
const GB = 1_000_000_000;

/** The space measured. */
const SPACE = schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000a1");

/** The account paying for most of the space's readings. */
const BUYER = "account-019f5530-8000-7000-8000-000000000302";

/** Another account paying for one reading. */
const PAYER = "account-019f5530-8000-7000-8000-000000000303";

/** The disk meter, by its declaration key. */
const DISK = "package-019f5530-8000-7000-8000-000000000401/disk.bytes";

/** The machine hours meter, by its declaration key. */
const UPTIME = "package-019f5530-8000-7000-8000-000000000401/machine.hours";

/** The SKU rating the disk meter, by its declaration key. */
const DISK_SKU = "package-019f5530-8000-7000-8000-000000000402/disk";

/** The SKU rating machine hours, by its declaration key. */
const UPTIME_SKU = "package-019f5530-8000-7000-8000-000000000402/uptime";

test("fold a page of readings into each payer's uses, a counter's amount as read and a level as its share of the month", async () => {
    // read 100 GB held an hour and 3 machine hours for the buyer, and 2 machine hours for another payer
    const { crypto } = testSimulation().environment("machine");
    const level = 100 * GB;
    const read = (
        id: string,
        meter: string,
        sku: string,
        payerId: string,
        value: number,
        data: object,
    ) => ({
        scope: SPACE,
        id,
        time: EventTime.of(START + HOUR),
        keys: { meter, installationId: null, sku, payerId, value },
        data,
    });
    const folded = await Reading.uses(
        {
            scope: SPACE,
            claimedAt: START + 2 * HOUR,
            events: [
                read("1", DISK, DISK_SKU, BUYER, level * HOUR, { level }),
                read("2", UPTIME, UPTIME_SKU, BUYER, 3, {}),
                read("3", UPTIME, UPTIME_SKU, PAYER, 2, {}),
            ],
            after: 0,
            through: 3,
        },
        crypto,
    );

    // give each payer its own uses, keyed by the page's claim and readings, as a retry repeats them
    const use = async (
        payer: string,
        meter: string,
        sku: string,
        quantity: number,
        isLevel: boolean,
        reading: string,
    ) => ({
        id: await usage.derive(
            SPACE,
            JSON.stringify([payer, meter, sku, null, START, reading]),
            crypto,
        ),
        time: EventTime.of(START + HOUR),
        meter,
        sku,
        spaceId: SPACE,
        machineId: null,
        installationId: null,
        quantity,
        level: isLevel ? level : null,
        data: { from: START + HOUR, to: START + HOUR },
    });
    const request = (payer: string, readings: string[]) =>
        RequestId.derive(START + 2 * HOUR, JSON.stringify([payer, ...readings]), crypto);
    expect(folded).toEqual([
        {
            accountId: BUYER,
            requestId: await request(BUYER, ["1", "2"]),
            items: [
                await use(BUYER, DISK, DISK_SKU, (level * HOUR) / OCTOBER, true, "1"),
                await use(BUYER, UPTIME, UPTIME_SKU, 3, false, "2"),
            ],
        },
        {
            accountId: PAYER,
            requestId: await request(PAYER, ["3"]),
            items: [await use(PAYER, UPTIME, UPTIME_SKU, 2, false, "3")],
        },
    ]);
});
