/** The bytes in a megabyte. */
const MB = 1_000_000;

/** The bytes in a gigabyte. */
const GB = 1_000_000_000;

/** The CI minutes a first developer runs a month: 20 runs a day of 10 minutes over 22 workdays. */
const CI_MINUTES = 4_400;

/** The CI minutes each further developer adds a month: 7 runs a day of 10 minutes, 22 days. */
const CI_MINUTES_PER_DEVELOPER = 1_540;

/** The memory a CI run holds while it runs, in GB, well under the 7 GB runner GitHub reserves. */
const CI_MEMORY = 2;

/** GitHub Team's price for a seat a month, in US dollars. */
const SEAT_GITHUB = 4;

/** The Linux Actions minutes GitHub Team includes a month. */
const ACTIONS_INCLUDED = 3_000;

/** GitHub's price for a Linux 2-core runner minute in US dollars, as of January 2026. */
const ACTIONS_RATE = 0.006;

/** The attachments a person adds to issues a month: 80 screenshots of 300 KB, six 20 MB clips. */
const ATTACHMENTS = 150 * MB;

/** The emails an app sends each user a month: a sign-in link, a digest and a notice. */
const EMAILS_PER_USER = 3;

/** The analytics events each user sends a month: about two sessions a week of six events. */
const EVENTS_PER_USER = 50;

/** The workflow actions each user starts a month: a few jobs of about five steps. */
const ACTIONS_PER_USER = 20;

/** The bytes each user is served a month: about forty page loads of half a megabyte. */
const TRANSFER_PER_USER = 20 * MB;

/** The requests each user makes a month: about forty page loads of seven requests. */
const REQUESTS_PER_USER = 300;

/** The size of a team and of its app's audience a bill is read for. */
export type Scale = { people: number; users: number };

/** What an item costs a month. */
export type Price =
    | { kind: "fee"; monthlyAt: (scale: Scale) => number; basis: string }
    | { kind: "bundled"; basis: string }
    | { kind: "included" };

/** Return a monthly fee charged for every person on the plan. */
function seat(monthly: number, basis: string): Price {
    return { kind: "fee", monthlyAt: (scale) => monthly * scale.people, basis };
}

/** Return a monthly fee charged once, however many people use it. */
export function flat(monthly: number, basis: string): Price {
    return { kind: "fee", monthlyAt: () => monthly, basis };
}

/** Return a fee that follows the team and its audience, by a vendor's published meters. */
function metered(basis: string, monthlyAt: (scale: Scale) => number): Price {
    return { kind: "fee", monthlyAt, basis };
}

/** The monthly list prices of the tools the figures rent, by subscription. */
export const prices: Readonly<Record<string, Price>> = {
    Notion: seat(12, "Plus"),
    Linear: seat(12, "Basic"),
    Slack: seat(8.75, "Pro"),
    Discord: flat(0, "Free"),
    GitHub: metered("Team + Actions", ({ people }) => {
        const minutes = ciMinutesAt(people);

        return SEAT_GITHUB * people + Math.max(0, minutes - ACTIONS_INCLUDED) * ACTIONS_RATE;
    }),
    Framer: flat(15, "Basic site"),
    Calendly: seat(12, "Standard"),
    Replit: seat(20, "Core"),
    "Google Workspace": seat(8.4, "Business Starter"),
    "Microsoft 365": seat(8.4, "Business Basic"),
    WorkOS: metered("Free to 1M users", ({ users }) =>
        users <= 1_000_000 ? 0 : Math.ceil((users - 1_000_000) / 1_000_000) * 2_500,
    ),
    Doppler: seat(21, "Team"),
    Supabase: metered("Pro + usage", ({ users }) => {
        const database = (users * workloadOf("Supabase").perUser.database) / GB;

        return 25 + Math.max(0, users - 100_000) * 0.003_25 + Math.max(0, database - 8) * 0.125;
    }),
    Dropbox: seat(11.99, "Plus"),
    Elastic: metered(
        "Standard, sized to the index",
        ({ users }) => 99 * Math.max(1, users / 100_000),
    ),
    Resend: metered("Pro or Scale", ({ users }) => resendAt(users * EMAILS_PER_USER)),
    Temporal: metered("Essentials + actions", ({ users }) => {
        const actions = users * ACTIONS_PER_USER;

        return 100 + Math.max(0, actions - 1_000_000) * 0.000_05;
    }),
    PostHog: metered("Free to 1M events", ({ users }) => postHogAt(users * EVENTS_PER_USER)),
    Vercel: metered("Pro + transfer", ({ people, users }) => {
        const transfer = (users * TRANSFER_PER_USER) / GB;
        const requests = users * REQUESTS_PER_USER;

        return (
            20 * people +
            Math.max(0, transfer - 1_000) * 0.15 +
            Math.max(0, requests - 10_000_000) * 0.000_002
        );
    }),
    Superhuman: seat(30, "Starter"),
    Confluence: seat(5.16, "Standard"),
    Obsidian: seat(5, "Sync"),
    ClickUp: seat(10, "Unlimited"),
    Box: seat(14, "Personal Pro"),
    Excalidraw: seat(6, "Excalidraw+"),
    FigJam: seat(5, "Professional"),
    Mural: seat(12, "Team+"),
    Lucid: seat(9, "Lucidspark Individual"),
    Thunderbird: flat(0, "Free"),
    DeepSeek: flat(0, "Free"),
    Make: flat(10.59, "Core"),
    "Cal.com": seat(15, "Teams"),
    WhatsApp: flat(0, "Free"),
    "Proton Mail": seat(4.99, "Mail Plus"),
    iCloud: seat(2.99, "iCloud+ 200 GB"),
    Coda: seat(16, "Pro"),
    Jira: seat(7.91, "Standard"),
    Trello: seat(6, "Standard"),
    Asana: seat(13.49, "Starter"),
    Airtable: seat(24, "Team"),
    Miro: seat(10, "Starter"),
    Zapier: flat(29.99, "Professional"),
    n8n: flat(24, "Starter"),
    ChatGPT: seat(20, "Plus"),
    Claude: seat(20, "Pro"),
    Gemini: seat(19.99, "Google AI Pro"),
    Grok: seat(30, "SuperGrok"),
};

/** The users the figures that count people read a bill for: an app a few months into its launch. */
export const BASE_USERS = 1_000;

/** The work a tool's job takes a month: in database and file bytes, GB-hours and requests. */
export type Usage = { database: number; files: number; compute: number; requests: number };

/** The work a tool's job takes: once for the team, per person, and per user. */
export type Workload = { shared: Usage; perPerson: Usage; perUser: Usage; isAtLeast: boolean };

/** No work at all. */
const none: Usage = { database: 0, files: 0, compute: 0, requests: 0 };

/** Return a workload from the parts of it a tool has. */
function workloadFrom(parts: {
    shared?: Partial<Usage>;
    perPerson?: Partial<Usage>;
    perUser?: Partial<Usage>;
    isAtLeast?: boolean;
}): Workload {
    return {
        shared: { ...none, ...parts.shared },
        perPerson: { ...none, ...parts.perPerson },
        perUser: { ...none, ...parts.perUser },
        isAtLeast: parts.isAtLeast ?? false,
    };
}

/** Return the workload of a tool each person uses alone, such as their docs or mail. */
function personal(usage: Partial<Usage>): Workload {
    return workloadFrom({ perPerson: usage });
}

/** The work each tool's job takes a few months into a launch, by tool. */
export const workloads: Readonly<Record<string, Workload>> = {
    Notion: personal({ database: 141 * MB, files: 94 * MB, compute: 0.01 }),
    Linear: personal({ database: 22 * MB, files: ATTACHMENTS, compute: 0.01 }),
    Slack: personal({ database: 326 * MB, files: 489 * MB, compute: 0.5 }),
    Discord: personal({ database: 160 * MB, files: 240 * MB, compute: 0.3 }),
    Calendly: personal({ database: 0.6 * MB, files: 0, compute: 0.005 }),
    Gmail: personal({ database: 400 * MB, files: 7.6 * GB, compute: 0.2 }),
    Dropbox: personal({ database: 600 * MB, files: 59.4 * GB, compute: 0.05 }),
    Superhuman: personal({ database: 400 * MB, files: 7.6 * GB, compute: 0.2 }),
    Jira: personal({ database: 22 * MB, files: ATTACHMENTS, compute: 0.01 }),
    Trello: personal({ database: 15 * MB, files: ATTACHMENTS, compute: 0.01 }),
    Asana: personal({ database: 22 * MB, files: ATTACHMENTS, compute: 0.01 }),
    Airtable: personal({ database: 12 * MB, files: 100 * MB, compute: 0.02 }),
    Miro: personal({ database: 30 * MB, files: 30 * MB, compute: 0.05 }),
    Zapier: personal({ database: 1 * MB, files: 0, compute: 0.05 }),
    Calendar: personal({ database: 1 * MB, files: 0, compute: 0.005 }),
    Drive: personal({ database: 150 * MB, files: 14850 * MB, compute: 0.05 }),
    Docs: personal({ database: 12 * MB, files: 80 * MB, compute: 0.01 }),
    Confluence: personal({ database: 24 * MB, files: 100 * MB, compute: 0.01 }),
    Obsidian: personal({ database: 45 * MB, files: 105 * MB, compute: 0.005 }),
    ClickUp: personal({ database: 27 * MB, files: ATTACHMENTS, compute: 0.01 }),
    Box: personal({ database: 400 * MB, files: 39.6 * GB, compute: 0.05 }),
    Excalidraw: personal({ database: 10 * MB, files: 10 * MB, compute: 0.01 }),
    FigJam: personal({ database: 30 * MB, files: 30 * MB, compute: 0.03 }),
    Mural: personal({ database: 30 * MB, files: 30 * MB, compute: 0.04 }),
    Lucid: personal({ database: 20 * MB, files: 20 * MB, compute: 0.03 }),
    Thunderbird: personal({ database: 300 * MB, files: 5.7 * GB, compute: 0.2 }),
    DeepSeek: personal({ database: 15 * MB, files: 0, compute: 0 }),
    "Power Automate": personal({ database: 1 * MB, files: 0, compute: 0.05 }),
    "Outlook Calendar": personal({ database: 1 * MB, files: 0, compute: 0.005 }),
    Make: personal({ database: 1 * MB, files: 0, compute: 0.05 }),
    Word: personal({ database: 9 * MB, files: 80 * MB, compute: 0.01 }),
    Excel: personal({ database: 5 * MB, files: 30 * MB, compute: 0.01 }),
    "Cal.com": personal({ database: 0.6 * MB, files: 0, compute: 0.005 }),
    Teams: personal({ database: 360 * MB, files: 540 * MB, compute: 0.5 }),
    "Google Chat": personal({ database: 150 * MB, files: 150 * MB, compute: 0.2 }),
    WhatsApp: personal({ database: 100 * MB, files: 400 * MB, compute: 0.1 }),
    Outlook: personal({ database: 400 * MB, files: 7.6 * GB, compute: 0.2 }),
    "Proton Mail": personal({ database: 150 * MB, files: 2850 * MB, compute: 0.1 }),
    OneDrive: personal({ database: 150 * MB, files: 14850 * MB, compute: 0.05 }),
    iCloud: personal({ database: 500 * MB, files: 49.5 * GB, compute: 0.05 }),
    Coda: personal({ database: 12 * MB, files: 60 * MB, compute: 0.02 }),
    Sheets: personal({ database: 7.2 * MB, files: 10 * MB, compute: 0.01 }),
    n8n: personal({ database: 1 * MB, files: 0, compute: 0.1 }),
    ChatGPT: personal({ database: 15 * MB, files: 0, compute: 0 }),
    Claude: personal({ database: 15 * MB, files: 0, compute: 0 }),
    Gemini: personal({ database: 15 * MB, files: 0, compute: 0 }),
    Grok: personal({ database: 20 * MB, files: 0, compute: 0 }),
    GitHub: workloadFrom({
        shared: {
            database: 100 * MB,
            files: 2.85 * GB,
            compute: ((ciMinutesAt(1) - CI_MINUTES_PER_DEVELOPER) / 60) * CI_MEMORY,
        },
        perPerson: { database: 50 * MB, compute: (CI_MINUTES_PER_DEVELOPER / 60) * CI_MEMORY },
    }),
    Framer: workloadFrom({
        shared: { database: 33 * MB, files: 77 * MB },
        perUser: { compute: 0.000_02, requests: REQUESTS_PER_USER },
    }),
    Replit: workloadFrom({
        shared: { files: 150 * MB },
        perUser: { database: 0.15 * MB, compute: 0.01, requests: REQUESTS_PER_USER },
        isAtLeast: true,
    }),
    Vercel: workloadFrom({
        shared: { database: 40 * MB, files: 1.96 * GB },
        perUser: { compute: 0.018, requests: REQUESTS_PER_USER },
        isAtLeast: true,
    }),
    Supabase: workloadFrom({
        perUser: {
            database: 0.405 * MB,
            files: 0.045 * MB,
            compute: 0.000_3,
            requests: REQUESTS_PER_USER,
        },
    }),
    Elastic: workloadFrom({ perUser: { database: 1.5 * MB, compute: 0.000_2 } }),
    PostHog: workloadFrom({ perUser: { database: 3 * MB, compute: 0.000_5 } }),
    Resend: workloadFrom({ perUser: { database: 0.05 * MB, files: 0.05 * MB, compute: 0.000_01 } }),
    Temporal: workloadFrom({ perUser: { database: 0.2 * MB, compute: 0.000_1 } }),
    WorkOS: workloadFrom({ perUser: { database: 0.025 * MB, compute: 0.000_01 } }),
    Doppler: workloadFrom({ shared: { database: 2 * MB, compute: 0.001 } }),
};

/** Return the CI minutes a repository runs a month for a team of developers. */
function ciMinutesAt(people: number): number {
    return CI_MINUTES + (people - 1) * CI_MINUTES_PER_DEVELOPER;
}

/** Return the work a workload takes for a team and its audience. */
export function usageAt(workload: Workload, scale: Scale): Usage {
    const at = (key: keyof Usage) =>
        workload.shared[key] +
        workload.perPerson[key] * scale.people +
        workload.perUser[key] * scale.users;

    return {
        database: at("database"),
        files: at("files"),
        compute: at("compute"),
        requests: at("requests"),
    };
}

/** Return the sum of workloads, such as the tools a row puts together. */
export function sumOf(tools: readonly Workload[]): Workload {
    const add = (part: "shared" | "perPerson" | "perUser"): Usage => ({
        database: tools.reduce((sum, each) => sum + each[part].database, 0),
        files: tools.reduce((sum, each) => sum + each[part].files, 0),
        compute: tools.reduce((sum, each) => sum + each[part].compute, 0),
        requests: tools.reduce((sum, each) => sum + each[part].requests, 0),
    });

    return {
        shared: add("shared"),
        perPerson: add("perPerson"),
        perUser: add("perUser"),
        isAtLeast: tools.some((each) => each.isAtLeast),
    };
}

/** Return what Resend charges for a month's emails: Pro, then Scale, then overage. */
function resendAt(emails: number): number {
    // pick the smallest plan that covers the emails, or the largest with overage
    const plans: readonly (readonly [emails: number, monthly: number])[] = [
        [50_000, 20],
        [100_000, 35],
        [200_000, 160],
        [500_000, 350],
        [1_000_000, 650],
        [1_500_000, 825],
        [2_500_000, 1_150],
    ];
    const plan = plans.find(([included]) => emails <= included);

    return plan === undefined ? 1_150 + (emails - 2_500_000) * 0.000_46 : plan[1];
}

/** Return what PostHog charges for a month's events: a free million, then falling rates. */
function postHogAt(events: number): number {
    // read the tiers, each priced up to its top
    const tiers: readonly (readonly [upTo: number, rate: number])[] = [
        [1_000_000, 0],
        [2_000_000, 0.000_05],
        [15_000_000, 0.000_034_3],
        [50_000_000, 0.000_029_5],
        [100_000_000, 0.000_021_8],
        [Number.POSITIVE_INFINITY, 0.000_015],
    ];

    // add each tier's share of the events at its rate
    let from = 0;
    let cost = 0;
    for (const [upTo, rate] of tiers) {
        cost += Math.max(0, Math.min(events, upTo) - from) * rate;
        from = upTo;
    }

    return cost;
}

/** Return the work a tool's job takes by its name, or refuse a name the list lacks. */
export function workloadOf(tool: string): Workload {
    const workload = workloads[tool];
    if (workload === undefined) {
        throw new Error(`no workload for ${tool}`);
    }

    return workload;
}

/** Return what a month's work costs at Destack's unit prices, requests priced by the million. */
export function usageCostOf(
    usage: Usage,
    rates: { database: number; files: number; compute: number; requests: number },
): number {
    // price each meter at its rate
    const database = (usage.database / GB) * rates.database;
    const files = (usage.files / GB) * rates.files;
    const requests = (usage.requests / 1_000_000) * rates.requests;

    return database + files + usage.compute * rates.compute + requests;
}

/** Return the price of a subscription by its name, or refuse a name the price list lacks. */
export function priceOf(subscription: string): Price {
    const price = prices[subscription];
    if (price === undefined) {
        throw new Error(`no price for ${subscription}`);
    }

    return price;
}

/** Sum the monthly fees of prices for a team, leaving out what is charged by use or bundled. */
export function monthlyOf(lines: readonly Price[], scale: Scale): number {
    return lines.reduce((sum, line) => sum + (line.kind === "fee" ? line.monthlyAt(scale) : 0), 0);
}
