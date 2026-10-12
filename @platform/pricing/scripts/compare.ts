import {
    MACHINE_COMPARABLES,
    MACHINE_HOST,
    MACHINES,
    type PricingModel,
    RESOURCES,
    UNIT_PRICING,
    UNIT_SKUS,
    type Workload,
    WORKLOADS,
} from "../src/price/index.ts";
import { HYPERSCALERS, VENDOR_PRICING } from "../src/vendor/vendor.ts";

/** The decimals a price per pricing unit is printed with. */
const DECIMALS = 6;

/** The percent in one. */
const PERCENT = 100;

/** The US cents in a dollar. */
const CENTS = 100;

// print each unit's price, its Cloudflare cost and margin, and each hyperscaler's price on its meter
/** The table's heading. */
const HEADING = ["resource", "sku", "unit", "dollars per unit", "vendor", "margin or headroom"];

/** The table's rows, one per unit and vendor pricing its meter. */
const rows = UNIT_SKUS.flatMap((unit) => {
    // read the unit's price per pricing unit in dollars
    const { service, pricingUnit, unitSize, meter } = unit.definition;
    const price = UNIT_PRICING.price(meter, unitSize) / CENTS;
    const heading = [service, unit.name, pricingUnit, price.toFixed(DECIMALS), "", ""];

    // compare each vendor pricing the meter: Cloudflare's margin, the hyperscalers' headroom
    const compared = (["cloudflare", ...HYPERSCALERS] as const).flatMap((vendor) => {
        // leave out a vendor pricing no such meter
        const model = VENDOR_PRICING[vendor];
        const components = model.components(meter);
        if (components === undefined) {
            return [];
        }

        // price the vendor's components of one pricing unit
        const listed = model.price(meter, unitSize) / CENTS;
        const ratio = vendor === "cloudflare" ? 1 - listed / price : listed / price - 1;
        const names = components.map(([sku]) => sku.name).join(" + ");

        return [["", names, pricingUnit, listed.toFixed(DECIMALS), vendor, share(price, ratio)]];
    });

    return [heading, ...compared];
});

// print each dedicated machine's hourly price and monthly cap, its host's cost and margin, and each comparable's price
/** The rows of the dedicated machines, one per size and quote. */
const machines = Object.entries(MACHINES).flatMap(([size, offer]) => {
    // read the hourly price and the monthly cap in dollars
    const hourly = offer.hourly / CENTS;
    const fee = offer.monthly / CENTS;
    const heading = ["Machine", size, "hour", hourly.toFixed(DECIMALS), "", ""];
    const cap = ["", "cap", "month", fee.toFixed(DECIMALS), "", ""];

    // quote the host's margin and each comparable's headroom
    const host = MACHINE_HOST.quote(offer);
    const quoted = [
        ...(host === undefined ? [] : [[host, percent(1 - host.monthly / CENTS / fee)] as const]),
        ...MACHINE_COMPARABLES.flatMap((vendor) => {
            const quote = vendor.quote(offer);

            return quote === undefined
                ? []
                : [[quote, percent(quote.monthly / CENTS / fee - 1)] as const];
        }),
    ];

    return [
        heading,
        cap,
        ...quoted.map(([quote, margin]) => [
            "",
            quote.name,
            "month",
            (quote.monthly / CENTS).toFixed(DECIMALS),
            new URL(quote.source).hostname,
            margin,
        ]),
    ];
});
rows.push(...machines);

// print each resource's reference month at the units, its Cloudflare margin and each hyperscaler's headroom
/** The rows of the resources' reference months, one per resource and vendor pricing it. */
const months = RESOURCES.flatMap((resource) => {
    // price the month at the units in dollars
    const workload = WORKLOADS[resource];
    const price = workload.price(UNIT_PRICING) / CENTS;
    const heading = [resource, "reference month", "month", price.toFixed(DECIMALS), "", ""];

    // price it at each vendor pricing all its meters: Cloudflare's margin, the hyperscalers' headroom
    const compared = (["cloudflare", ...HYPERSCALERS] as const).flatMap((vendor) => {
        // leave out a vendor missing one of the month's meters
        const model = VENDOR_PRICING[vendor];
        if (!prices(model, workload)) {
            return [];
        }

        // price the month at the vendor in dollars
        const listed = workload.price(model) / CENTS;
        const ratio = vendor === "cloudflare" ? 1 - listed / price : listed / price - 1;

        return [["", "month", "month", listed.toFixed(DECIMALS), vendor, share(price, ratio)]];
    });

    return [heading, ...compared];
});
rows.push(...months);

// align the columns and print them
/** The widest cell of each column. */
const widths = [HEADING, ...rows].reduce<number[]>(
    (widest, row) => row.map((cell, index) => Math.max(widest[index] ?? 0, cell.length)),
    [],
);
for (const row of [HEADING, ...rows]) {
    process.stdout.write(
        `${row.map((cell, index) => cell.padEnd(widths[index] ?? 0)).join("  ")}\n`,
    );
}

/** Check whether a pricing model prices every meter of a workload. */
function prices(model: PricingModel, workload: Workload): boolean {
    return workload.quantities.every(([meter]) => model.components(meter.reference) !== undefined);
}

/** Write a share of a price, free when the price is nothing. */
function share(price: number, ratio: number): string {
    return price === 0 ? "free" : percent(ratio);
}

/** Write a ratio as a whole percent with its sign. */
function percent(ratio: number): string {
    const rounded = Math.round(ratio * PERCENT);

    return `${rounded > 0 ? "+" : ""}${rounded}%`;
}
