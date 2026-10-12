import { MACHINE_SIZES } from "@destack/account/object";
import { Rate } from "@destack/finance";
import { METERS } from "@destack/observability/meter";
import { expect, test } from "@destack/test";
import { CLOUDFLARE_PRICING } from "../src/vendor/cloudflare.ts";
import {
    MACHINE_SKUS,
    RESOURCES,
    TYPICAL_APP,
    TYPICAL_WORKLOADS,
    UNIT_PRICING,
} from "../src/index.ts";

/** The hours of a 31 day month, past every machine's cap. */
const MONTH_HOURS = 744;

/** The units in a million, the quantity each meter is priced at. */
const MILLION = 1_000_000;

test("price every meter at one unit and at Cloudflare's SKUs", () => {
    // read each meter's unit, its Cloudflare components, and both prices of a million in US cents
    const priced = Object.fromEntries(
        METERS.map((meter) => [
            meter.name,
            [
                UNIT_PRICING.components(meter.reference)?.map(([sku, quantity]) => [
                    sku.name,
                    quantity,
                ]),
                CLOUDFLARE_PRICING.components(meter.reference)?.map(([sku, quantity]) => [
                    sku.name,
                    quantity,
                ]),
                UNIT_PRICING.price(meter.reference, MILLION),
                CLOUDFLARE_PRICING.price(meter.reference, MILLION),
            ],
        ]),
    );

    expect(priced).toEqual({
        "compute.memory": [
            [["destack.compute.memory", 1]],
            [["cloudflare.durableObjectDuration", 1]],
            1666.666668,
            1250,
        ],
        "compute.cpu": [[["destack.compute.cpu", 1]], [["cloudflare.workerCpu", 1]], 3, 2],
        "compute.requests": [
            [["destack.compute.requests", 1]],
            [
                ["cloudflare.workerRequests", 1],
                ["cloudflare.durableObjectRequests", 1],
            ],
            60,
            45,
        ],
        "database.bytes": [
            [["destack.database.bytes", 1]],
            [["cloudflare.durableObjectStorage", 1]],
            0.025,
            0.02,
        ],
        "database.rows.read": [
            [["destack.database.rows.read", 1]],
            [["cloudflare.durableObjectRowsRead", 1]],
            0.125,
            0.1,
        ],
        "database.rows.written": [
            [["destack.database.rows.written", 1]],
            [["cloudflare.durableObjectRowsWritten", 1]],
            125,
            100,
        ],
        "storage.bytes": [
            [["destack.storage.bytes", 1]],
            [["cloudflare.r2Storage", 1]],
            0.002,
            0.0015,
        ],
        "storage.writes": [[["destack.storage.writes", 1]], [["cloudflare.r2ClassA", 1]], 600, 450],
        "storage.reads": [[["destack.storage.reads", 1]], [["cloudflare.r2ClassB", 1]], 50, 36],
        "network.egress": [[["destack.network.egress", 1]], [["cloudflare.r2Egress", 1]], 0, 0],
    });
});

test("price a typical app's month at the units, resource by resource", () => {
    // price each resource of the typical app in US cents
    const prices = Object.fromEntries(
        RESOURCES.map((resource) => [resource, TYPICAL_WORKLOADS[resource].price(UNIT_PRICING)]),
    );

    expect({ prices, total: TYPICAL_APP }).toEqual({
        prices: { Compute: 68.58, Database: 41.25, Storage: 13, Network: 0 },
        total: 123,
    });
});

test("bill each machine's awake hours at its hourly price, capped at 500 hours a month", () => {
    // rate each size's SKU past its cap, charging its monthly price
    const machines = Object.fromEntries(
        MACHINE_SIZES.map((size) => {
            const sku = MACHINE_SKUS[size];
            const rated = Rate.usage(sku, MONTH_HOURS, true, "1");

            return [
                size,
                [sku.definition.unitAmount, sku.definition.maximumQuantity, rated.listCost],
            ];
        }),
    );

    expect(machines).toEqual({
        small: ["5", 500, "2500"],
        medium: ["10", 500, "5000"],
        large: ["20", 500, "10000"],
        xlarge: ["40", 500, "20000"],
    });
});
