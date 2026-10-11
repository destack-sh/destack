/** What an item costs a month: a fee and how it adds up, a rate charged by use, a share of a subscription already billed, or nothing beyond your plan. */
export type Price =
    | { kind: "fee"; monthly: number; basis: string }
    | { kind: "usage"; basis: string }
    | { kind: "bundled"; basis: string }
    | { kind: "included" };

/** Return a monthly fee and how it adds up. */
function fee(monthly: number, basis: string): Price {
    return { kind: "fee", monthly, basis };
}

/** Return a rate charged by use. */
function usage(basis: string): Price {
    return { kind: "usage", basis };
}

/**
 * The monthly list prices of the tools the figures rent, for a team of three on monthly billing, by subscription.
 *
 * Read from the vendors' pricing pages on 2026-10-11, in US dollars, on the cheapest paid plan such a team needs.
 * Linear and Webflow publish only yearly billing, and Mailchimp and n8n only showed local currency, so those are the nearest published figures.
 */
export const prices: Readonly<Record<string, Price>> = {
    Notion: fee(36, "$12 × 3 seats"),
    Linear: fee(30, "$10 × 3 seats"),
    Slack: fee(26.25, "$8.75 × 3 seats"),
    GitHub: fee(12, "$4 × 3 seats"),
    Framer: fee(15, "$15 a site"),
    Calendly: fee(12, "$12, one host"),
    Replit: fee(20, "$20, one builder"),
    "Google Workspace": fee(25.2, "$8.40 × 3 seats"),
    Clerk: fee(25, "$25 flat"),
    WorkOS: usage("free to 1M users"),
    Doppler: fee(63, "$21 × 3 seats"),
    Supabase: fee(25, "$25 flat"),
    Dropbox: fee(54, "$18 × 3 seats"),
    Elastic: fee(99, "from $99"),
    Resend: fee(20, "$20 flat"),
    Temporal: usage("$50 per 1M actions"),
    PostHog: usage("free to 1M events"),
    Vercel: fee(60, "$20 × 3 seats"),
    Stripe: usage("2.9% + 30¢ a charge"),
    Figma: fee(60, "$20 × 3 seats"),
    Superhuman: fee(90, "$30 × 3 seats"),
    Granola: fee(42, "$14 × 3 seats"),
    Loom: fee(54, "$18 × 3 seats"),
    Jira: fee(23.73, "$7.91 × 3 seats"),
    Trello: fee(18, "$6 × 3 seats"),
    Asana: fee(40.47, "$13.49 × 3 seats"),
    Airtable: fee(72, "$24 × 3 seats"),
    Miro: fee(30, "$10 × 3 seats"),
    Zapier: fee(29.99, "$29.99 flat"),
    Webflow: fee(15, "$15 a site"),
    HubSpot: fee(60, "$20 × 3 seats"),
    Mailchimp: fee(20, "$20, 500 contacts"),
    n8n: fee(24, "$24 flat"),
    ChatGPT: fee(75, "$25 × 3 seats"),
    Claude: fee(75, "$25 × 3 seats"),
    AWS: usage("pay per use"),
};

/** The bytes in a kilobyte. */
const KB = 1000;

/** The bytes in a megabyte. */
const MB = 1_000_000;

/** The bytes in a gigabyte. */
const GB = 1_000_000_000;

/**
 * The bytes of data each tool holds for an individual or a small team a few months into a launch, by tool.
 *
 * Illustrative estimates: a workspace of a few hundred pages is megabytes, and mail, files, recordings and events run to hundreds of megabytes.
 */
export const sizes: Readonly<Record<string, number>> = {
    Notion: 9 * MB,
    Linear: 2 * MB,
    Slack: 180 * MB,
    GitHub: 40 * MB,
    Framer: 25 * MB,
    Calendly: 400 * KB,
    Replit: 12 * MB,
    Gmail: 650 * MB,
    Clerk: 3 * MB,
    WorkOS: 500 * KB,
    Doppler: 60 * KB,
    Supabase: 120 * MB,
    Dropbox: 900 * MB,
    Elastic: 60 * MB,
    Resend: 8 * MB,
    Temporal: 40 * MB,
    PostHog: 700 * MB,
    Vercel: 150 * MB,
    Stripe: 2 * MB,
    Figma: 300 * MB,
    Superhuman: 650 * MB,
    Granola: 20 * MB,
    Loom: 1.5 * GB,
    Jira: 3 * MB,
    Trello: 2 * MB,
    Asana: 3 * MB,
    Airtable: 15 * MB,
    Miro: 60 * MB,
    Zapier: 1 * MB,
    Calendar: 1 * MB,
    Drive: 800 * MB,
    Docs: 20 * MB,
    Sheets: 8 * MB,
    Webflow: 30 * MB,
    HubSpot: 25 * MB,
    Mailchimp: 5 * MB,
    n8n: 1 * MB,
    Forms: 2 * MB,
    ChatGPT: 15 * MB,
    Claude: 15 * MB,
    AWS: 2 * GB,
};

/** Return the bytes a tool holds by its name, or refuse a name the list lacks. */
export function sizeOf(tool: string): number {
    const size = sizes[tool];
    if (size === undefined) {
        throw new Error(`no size for ${tool}`);
    }

    return size;
}

/** Return bytes as a bill shows them: kilobytes, megabytes or gigabytes, whole below ten and to one place above a gigabyte. */
export function bytesOf(data: number): string {
    if (data < MB) {
        return `${Math.round(data / KB)} KB`;
    } else if (data < GB) {
        return `${Math.round(data / MB)} MB`;
    }

    return `${(data / GB).toFixed(1)} GB`;
}

/** Write the data total of a bill: the bytes summed. */
export function bytesTotalOf(data: readonly number[]): string {
    return bytesOf(data.reduce((sum, size) => sum + size, 0));
}

/** Return a sum of dollars as a bill shows it: whole dollars bare, anything else to the cent. */
export function dollarsOf(amount: number): string {
    const cents = Math.round(amount * 100);

    return cents % 100 === 0 ? `$${cents / 100}` : `$${(cents / 100).toFixed(2)}`;
}

/** Return the price of a subscription by its name, or refuse a name the price list lacks. */
export function priceOf(subscription: string): Price {
    const price = prices[subscription];
    if (price === undefined) {
        throw new Error(`no price for ${subscription}`);
    }

    return price;
}

/** Sum the monthly fees of prices, leaving out what is charged by use, bundled or included. */
export function monthlyOf(lines: readonly Price[]): number {
    return lines.reduce((sum, line) => sum + (line.kind === "fee" ? line.monthly : 0), 0);
}

/** Write the monthly total of a bill: the fees summed with any plan on top, marked when more is charged by use. */
export function billOf(lines: readonly Price[], plan: number): string {
    const amount = dollarsOf(monthlyOf(lines) + plan);
    const isMetered = lines.some((line) => line.kind === "usage");

    return isMetered ? `${amount} + usage` : amount;
}
