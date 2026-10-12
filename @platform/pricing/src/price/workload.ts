import type { Meter } from "@destack/finance";
import * as meters from "@destack/observability/meter";
import type { PricingModel } from "./model.ts";
import { RESOURCES, type Resource, UNIT_PRICING } from "./unit.ts";

/** A month of one resource's usage, as the quantity of each of its meters in the meter's unit. */
export class Workload {
    /** Each meter with its quantity, such as GB-seconds held or byte-months kept. */
    readonly quantities: readonly (readonly [Meter, number])[];

    /** Keep a resource's meters and their quantities. */
    constructor(quantities: readonly (readonly [Meter, number])[]) {
        this.quantities = quantities;
    }

    /** Price the workload's meters at a pricing model, in US cents. */
    price(model: PricingModel): number {
        return this.quantities.reduce(
            (total, [meter, quantity]) => total + model.price(meter.reference, quantity),
            0,
        );
    }
}

/** A month of each resource, priced at the units, at Cloudflare's cost and at AWS. */
export const WORKLOADS: Readonly<Record<Resource, Workload>> = {
    // a 128 MB instance held for 730 hours, its memory time alone
    Compute: new Workload([[meters.memory, 0.128 * 730 * 3600]]),

    // 1 GB kept, read ten times as often as written
    Database: new Workload([
        [meters.databaseBytes, 1_000_000_000],
        [meters.rowsRead, 10_000_000],
        [meters.rowsWritten, 1_000_000],
    ]),

    // 100,000 files of 1 MB, each written once and read ten times
    Storage: new Workload([
        [meters.storageBytes, 100_000_000_000],
        [meters.storageWrites, 100_000],
        [meters.storageReads, 1_000_000],
    ]),

    // 100 GB served
    Network: new Workload([[meters.egress, 100_000_000_000]]),
};

/** A typical app's month by resource: 128 MB busy two hours a day for 10,000 requests a day. */
export const TYPICAL_WORKLOADS: Readonly<Record<Resource, Workload>> = {
    // 128 MB held two hours a day, each request taking 5 CPU milliseconds
    Compute: new Workload([
        [meters.memory, 0.128 * 2 * 30 * 3600],
        [meters.cpu, 300_000 * 5],
        [meters.requests, 300_000],
    ]),

    // 100 MB kept, each request reading 33 rows and writing one
    Database: new Workload([
        [meters.databaseBytes, 100_000_000],
        [meters.rowsRead, 10_000_000],
        [meters.rowsWritten, 300_000],
    ]),

    // 1 GB of files, written 10,000 times and read 100,000 times
    Storage: new Workload([
        [meters.storageBytes, 1_000_000_000],
        [meters.storageWrites, 10_000],
        [meters.storageReads, 100_000],
    ]),

    // 10 GB served
    Network: new Workload([[meters.egress, 10_000_000_000]]),
};

/** A typical app's monthly price at the units in whole US cents, which the pricing page shows. */
export const TYPICAL_APP = Math.round(
    RESOURCES.reduce(
        (total, resource) => total + TYPICAL_WORKLOADS[resource].price(UNIT_PRICING),
        0,
    ),
);
