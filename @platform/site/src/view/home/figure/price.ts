/** What an item costs a month. */
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

/** The monthly list prices of the tools the figures rent, for one person, by subscription. */
export const prices: Readonly<Record<string, Price>> = {
    Notion: fee(12, "Plus"),
    Linear: fee(12, "Basic"),
    Slack: fee(8.75, "Pro"),
    Discord: fee(0, "Free"),
    GitHub: fee(4, "Team"),
    Framer: fee(15, "Basic site"),
    Calendly: fee(12, "Standard"),
    Replit: fee(20, "Core"),
    "Google Workspace": fee(8.4, "Business Starter"),
    "Microsoft 365": fee(8.4, "Business Basic"),
    WorkOS: usage("free to 1M users"),
    Doppler: fee(21, "Team"),
    Supabase: fee(25, "Pro"),
    Dropbox: fee(11.99, "Plus"),
    Elastic: fee(99, "Standard, from"),
    Resend: fee(20, "Pro"),
    Temporal: usage("$50 per 1M actions"),
    PostHog: usage("free to 1M events"),
    Vercel: fee(20, "Pro"),
    Superhuman: fee(30, "Starter"),
    Confluence: fee(5.16, "Standard"),
    Obsidian: fee(5, "Sync"),
    ClickUp: fee(10, "Unlimited"),
    Box: fee(14, "Personal Pro"),
    Excalidraw: fee(6, "Excalidraw+"),
    tldraw: fee(0, "Free"),
    Make: fee(10.59, "Core"),
    "Cal.com": fee(15, "Teams"),
    WhatsApp: fee(0, "Free"),
    "Proton Mail": fee(4.99, "Mail Plus"),
    iCloud: fee(2.99, "iCloud+ 200 GB"),
    Coda: fee(16, "Pro"),
    Jira: fee(7.91, "Standard"),
    Trello: fee(6, "Standard"),
    Asana: fee(13.49, "Starter"),
    Airtable: fee(24, "Team"),
    Miro: fee(10, "Starter"),
    Zapier: fee(29.99, "Professional"),
    Mailchimp: fee(20, "Standard, 500 contacts"),
    n8n: fee(24, "Starter"),
    ChatGPT: fee(20, "Plus"),
    Claude: fee(20, "Pro"),
    Gemini: fee(19.99, "Google AI Pro"),
    Grok: fee(30, "SuperGrok"),
};

/** The bytes in a megabyte. */
const MB = 1_000_000;

/** The bytes in a gigabyte. */
const GB = 1_000_000_000;

/** The work a tool's job takes a month, the same whoever runs it. */
export type Workload = { storage: number; compute: number; isAtLeast: boolean };

/** The work each tool's job takes for one person a few months into a launch, by tool. */
export const workloads: Readonly<Record<string, Workload>> = {
    Notion: { storage: 235 * MB, compute: 0.01, isAtLeast: false },
    Linear: { storage: 25 * MB, compute: 0.01, isAtLeast: false },
    Slack: { storage: 815 * MB, compute: 0.5, isAtLeast: false },
    Discord: { storage: 400 * MB, compute: 0.3, isAtLeast: false },
    GitHub: { storage: 3 * GB, compute: 58, isAtLeast: true },
    Framer: { storage: 110 * MB, compute: 0.02, isAtLeast: false },
    Calendly: { storage: 0.6 * MB, compute: 0.005, isAtLeast: false },
    Replit: { storage: 300 * MB, compute: 10, isAtLeast: true },
    Gmail: { storage: 8 * GB, compute: 0.2, isAtLeast: false },
    WorkOS: { storage: 25 * MB, compute: 0.01, isAtLeast: false },
    Doppler: { storage: 2 * MB, compute: 0.001, isAtLeast: false },
    Supabase: { storage: 450 * MB, compute: 0.3, isAtLeast: false },
    Dropbox: { storage: 60 * GB, compute: 0.05, isAtLeast: false },
    Elastic: { storage: 1.5 * GB, compute: 0.2, isAtLeast: false },
    Resend: { storage: 100 * MB, compute: 0.01, isAtLeast: false },
    Temporal: { storage: 200 * MB, compute: 0.1, isAtLeast: false },
    PostHog: { storage: 3 * GB, compute: 0.5, isAtLeast: false },
    Vercel: { storage: 2 * GB, compute: 18, isAtLeast: true },
    Superhuman: { storage: 8 * GB, compute: 0.2, isAtLeast: false },
    Jira: { storage: 3 * MB, compute: 0.01, isAtLeast: false },
    Trello: { storage: 2 * MB, compute: 0.01, isAtLeast: false },
    Asana: { storage: 3 * MB, compute: 0.01, isAtLeast: false },
    Airtable: { storage: 15 * MB, compute: 0.02, isAtLeast: false },
    Miro: { storage: 60 * MB, compute: 0.05, isAtLeast: false },
    Zapier: { storage: 1 * MB, compute: 0.05, isAtLeast: false },
    Calendar: { storage: 1 * MB, compute: 0.005, isAtLeast: false },
    Drive: { storage: 15 * GB, compute: 0.05, isAtLeast: false },
    Docs: { storage: 20 * MB, compute: 0.01, isAtLeast: false },
    Confluence: { storage: 40 * MB, compute: 0.01, isAtLeast: false },
    Obsidian: { storage: 150 * MB, compute: 0.005, isAtLeast: false },
    ClickUp: { storage: 30 * MB, compute: 0.01, isAtLeast: false },
    Box: { storage: 40 * GB, compute: 0.05, isAtLeast: false },
    Excalidraw: { storage: 20 * MB, compute: 0.01, isAtLeast: false },
    tldraw: { storage: 20 * MB, compute: 0.01, isAtLeast: false },
    Make: { storage: 1 * MB, compute: 0.05, isAtLeast: false },
    Word: { storage: 30 * MB, compute: 0.01, isAtLeast: false },
    Excel: { storage: 10 * MB, compute: 0.01, isAtLeast: false },
    "Cal.com": { storage: 0.6 * MB, compute: 0.005, isAtLeast: false },
    Teams: { storage: 900 * MB, compute: 0.5, isAtLeast: false },
    "Google Chat": { storage: 300 * MB, compute: 0.2, isAtLeast: false },
    WhatsApp: { storage: 500 * MB, compute: 0.1, isAtLeast: false },
    Outlook: { storage: 8 * GB, compute: 0.2, isAtLeast: false },
    "Proton Mail": { storage: 3 * GB, compute: 0.1, isAtLeast: false },
    OneDrive: { storage: 15 * GB, compute: 0.05, isAtLeast: false },
    iCloud: { storage: 50 * GB, compute: 0.05, isAtLeast: false },
    Coda: { storage: 15 * MB, compute: 0.02, isAtLeast: false },
    Sheets: { storage: 8 * MB, compute: 0.01, isAtLeast: false },
    Mailchimp: { storage: 5 * MB, compute: 0.02, isAtLeast: false },
    n8n: { storage: 1 * MB, compute: 0.1, isAtLeast: false },
    ChatGPT: { storage: 15 * MB, compute: 0, isAtLeast: false },
    Claude: { storage: 15 * MB, compute: 0, isAtLeast: false },
    Gemini: { storage: 15 * MB, compute: 0, isAtLeast: false },
    Grok: { storage: 20 * MB, compute: 0, isAtLeast: false },
};

/** Return the work a tool's job takes by its name, or refuse a name the list lacks. */
export function workloadOf(tool: string): Workload {
    const workload = workloads[tool];
    if (workload === undefined) {
        throw new Error(`no workload for ${tool}`);
    }

    return workload;
}

/** Return what a workload costs a month at Destack's unit prices for storage and compute. */
export function usageCostOf(
    workload: Workload,
    rates: { storage: number; compute: number },
): number {
    return (workload.storage / GB) * rates.storage + workload.compute * rates.compute;
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
