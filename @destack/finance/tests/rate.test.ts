import { expect, test } from "@destack/test";
import { defineSku } from "../src/declare/index.ts";
import { CREDITS, Rate } from "../src/rate/index.ts";
import { disk, storage } from "./fixture/storage.ts";

/** A gigabyte, as storage prices count it. */
const GB = 1_000_000_000;

/** Disk listed at 3 cents a GB-month. */
const listed = defineSku(
    {
        name: "cloudflare.r2.listed",
        description: "Disk storage at one list price.",
        provider: "cloudflare",
        service: "R2",
        category: "Storage",
        meter: disk.reference,
        pricingUnit: "GB-month",
        unitSize: GB,
        currency: "USD",
        unitAmount: "3",
        source: "https://developers.cloudflare.com/r2/pricing/",
        observedAt: "2026-10-06",
    },
    { package: storage },
);

/** Disk listed at 3 credit-cents a GB-month, which each billing currency prices through the seller's book. */
const credited = defineSku(
    {
        name: "destack.credited",
        description: "Disk storage priced in credits.",
        provider: "cloudflare",
        service: "R2",
        category: "Storage",
        meter: disk.reference,
        pricingUnit: "GB-month",
        unitSize: GB,
        currency: CREDITS,
        unitAmount: "3",
        source: "https://destack.sh/pricing",
        observedAt: "2026-10-07",
    },
    { package: storage },
);

test("price usage at the SKU's list price converted into the billing currency and rounded up, billing it within a billed subscription and nothing outside one", () => {
    const rated = [
        Rate.usage(listed, 500 * GB, true, "1"),
        Rate.usage(listed, GB / 3, true, "1"),
        Rate.usage(listed, 500 * GB, false, "1"),
        Rate.usage(credited, 500 * GB, true, "1.4"),
    ].map((each) => [each.skuPriceId, each.listUnitPrice, each.listCost, each.billedCost]);

    // a third of a GB-month keeps millionths of its quantity, and a credit at 1.40 Canadian cents prices 3 credit-cents at 4.2 cents
    const priced = `${storage.id}/cloudflare.r2.listed@2026-10-06`;
    expect(rated).toEqual([
        [priced, "3", "1500", "1500"],
        [priced, "3", "0.999999", "0.999999"],
        [priced, "3", "1500", "0"],
        [`${storage.id}/destack.credited@2026-10-07`, "4.2", "2100", "2100"],
    ]);
});
