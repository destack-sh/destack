import { fileURLToPath } from "node:url";
import type { Network } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { Cost } from "@destack/finance";
import { CLOUDFLARE_SKUS } from "../src/vendor/cloudflare.ts";

/** The US cents in a dollar. */
const CENTS = 100n;

/** Where a Cloudflare SKU's list price is read: a docs page's table row. */
interface Lookup {
    /** The page's path under the docs. */
    readonly page: string;
    /** The pattern capturing the price in dollars, or Free, in the page's Markdown. */
    readonly pattern: RegExp;
}

/** Where each Cloudflare SKU's price is read, by the SKU's name. */
const LOOKUPS: Readonly<Record<string, Lookup>> = {
    "cloudflare.durableObjectDuration": {
        page: "durable-objects/platform/pricing",
        pattern: /^\| Duration[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+)\/million GB-s/gmu,
    },
    "cloudflare.durableObjectRequests": {
        page: "durable-objects/platform/pricing",
        pattern: /^\| Requests \|[^|]*\|[^|]*\+ \$([\d.]+)\/million/gmu,
    },
    "cloudflare.durableObjectRowsRead": {
        page: "durable-objects/platform/pricing",
        pattern: /^\| Rows reads[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+) \/ million rows/gmu,
    },
    "cloudflare.durableObjectRowsWritten": {
        page: "durable-objects/platform/pricing",
        pattern: /^\| Rows written[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+) \/ million rows/gmu,
    },
    "cloudflare.durableObjectStorage": {
        page: "durable-objects/platform/pricing",
        pattern: /^\| SQL Stored data[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+)\/ GB-month/gmu,
    },
    "cloudflare.r2Storage": {
        page: "r2/pricing",
        pattern: /^\| Storage \| \$([\d.]+) \/ GB-month/gmu,
    },
    "cloudflare.r2ClassA": {
        page: "r2/pricing",
        pattern: /^\| Class A Operations \| \$([\d.]+) \/ million requests/gmu,
    },
    "cloudflare.r2ClassB": {
        page: "r2/pricing",
        pattern: /^\| Class B Operations \| \$([\d.]+) \/ million requests/gmu,
    },
    "cloudflare.r2Egress": {
        page: "r2/pricing",
        pattern: /^\| Egress \(data transfer to Internet\) \| (Free)/gmu,
    },
    "cloudflare.workerRequests": {
        page: "workers/platform/pricing",
        pattern: /^\| \*\*Standard\*\* \|[^|]*\+\$([\d.]+) per additional million \|/gmu,
    },
    "cloudflare.workerCpu": {
        page: "workers/platform/pricing",
        pattern: /\+\$([\d.]+) per additional million CPU milliseconds/gmu,
    },
};

/** The machine the refresh runs on. */
const environment = BunEnvironment.system;

/** The pages read so far, by URL, each read once. */
const pages = new Map<string, Promise<string>>();

// read every SKU's price in cents before writing anything
/** The day the prices are read, as YYYY-MM-DD. */
const today = new Date(environment.clock.now()).toISOString().slice(0, 10);
/** Each SKU with its price read in cents. */
const read = await Promise.all(
    CLOUDFLARE_SKUS.map(async (sku) => {
        // find where the SKU's price is published
        const lookup = LOOKUPS[sku.name];
        if (lookup === undefined) {
            throw new Error(`sku ${sku.name} has no docs lookup`);
        }

        // read it in dollars and write it in cents
        const dollars = await priceOf(sku.name, lookup, environment.network);

        return { sku, cents: Cost.of(Cost.scaled(dollars) * CENTS) };
    }),
);

// rewrite each amount and the day the prices were read
/** The Cloudflare module the refresh rewrites. */
const file = fileURLToPath(new URL("../src/vendor/cloudflare.ts", import.meta.url));
/** The module's text, each amount rewritten in turn. */
let module = await environment.files.readTextFile(file);
for (const { sku, cents } of read) {
    module = replaced(
        module,
        new RegExp(`(name: "${sku.name}",[^}]*?unitAmount: ")([^"]*)(")`, "u"),
        cents,
        sku.name,
    );
    if (cents !== sku.definition.unitAmount) {
        await environment.process.stdout.write(
            `${sku.name}: ${sku.definition.unitAmount} -> ${cents}\n`,
        );
    }
}
module = replaced(module, /(const OBSERVED_AT = ")([^"]*)(")/u, today, "cloudflare");
await environment.files.writeTextFile(file, module);

/** Read a Cloudflare price from its docs page, refusing a pattern matching never or differently. */
async function priceOf(name: string, lookup: Lookup, network: Network): Promise<string> {
    // read every match's captured price, a free one as zero
    const page = await text(`https://developers.cloudflare.com/${lookup.page}/index.md`, network);
    const prices = new Set(
        [...page.matchAll(lookup.pattern)].map((match) => (match[1] === "Free" ? "0" : match[1])),
    );
    const [price] = prices;
    if (price === undefined || prices.size > 1) {
        throw new Error(`sku ${name} reads ${prices.size} prices on ${lookup.page}`);
    }

    return price;
}

/** Replace a pattern's one match's middle group, refusing a source it misses. */
function replaced(source: string, pattern: RegExp, value: string, name: string): string {
    if (!pattern.test(source)) {
        throw new Error(`${name} has no amount to rewrite`);
    }

    return source.replace(
        pattern,
        (_match, before: string, _value: string, after: string) => `${before}${value}${after}`,
    );
}

/** Fetch a URL's text once, refusing a failed response. */
function text(url: string, network: Network): Promise<string> {
    const page = pages.get(url) ?? fetched(url, network).then((response) => response.text());
    pages.set(url, page);

    return page;
}

/** Fetch a URL, refusing a failed response. */
async function fetched(url: string, network: Network): Promise<Response> {
    const response = await network.fetch(new Request(url));
    if (!response.ok) {
        throw new Error(`${new URL(url).host}${new URL(url).pathname} answered ${response.status}`);
    }

    return response;
}
