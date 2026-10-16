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

/** GitHub Team's price for a seat a month in US dollars, as listed for its first 12 months. */
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

/** The requests each user makes a month: about forty page loads of seven requests. */
const REQUESTS_PER_USER = 300;

/** The requests a sign-in takes: about four sessions a month of three requests. */
const SIGN_IN_REQUESTS = 12;

/** The steps a workflow action takes. */
const STEPS_PER_ACTION = 5;

/** The workdays in a month. */
const WORKDAYS = 22;

/** The hours in a month, a year's 8,760 over twelve. */
const HOURS_PER_MONTH = 730;

/** The seconds in an hour. */
const SECONDS_PER_HOUR = 3600;

/** The units in a million, the quantity counted meters are priced per. */
const MILLION = 1_000_000;

/** The memory an instance holds in GB, the 128 MB that @platform/pricing's workloads hold. */
const INSTANCE_MEMORY = 0.128;

/** The seconds an instance stays awake after a request before it sleeps. */
const AWAKE_SECONDS = 10;

/** The people one instance serves at once, far fewer than the connections it can hold. */
const PEOPLE_PER_INSTANCE = 25;

/** The requests one instance serves a second at peak: a quarter of 200 at 5 CPU ms each. */
const REQUESTS_PER_SECOND = 50;

/** The busiest hour's load over the month's average. */
const PEAK_FACTOR = 4;

/** The CPU time a request takes in milliseconds, as @platform/pricing's typical app. */
const CPU_PER_REQUEST = 5;

/** The database rows a request reads, as @platform/pricing's typical app. */
const ROWS_READ_PER_REQUEST = 33;

/** The database rows a request writes, as @platform/pricing's typical app. */
const ROWS_WRITTEN_PER_REQUEST = 1;

/** The size of a file in bytes, as @platform/pricing's storage workload. */
const FILE_SIZE = MB;

/** The reads each file write brings, as @platform/pricing's storage workload. */
const FILE_READS_PER_WRITE = 10;

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

/** The list prices of the tools the figures rent on 2026-10-16, billed annually where offered. */
export const prices: Readonly<Record<string, Price>> = {
    // per seat, billed annually
    Notion: seat(10, "Plus, annual"),
    Linear: seat(10, "Basic, annual"),
    Slack: seat(7.25, "Pro, annual"),
    Calendly: seat(10, "Standard, annual"),
    Replit: seat(18, "Core, annual"),
    "Google Workspace": seat(7, "Business Starter, annual"),
    "Microsoft 365": seat(7, "Business Basic, annual"),
    Superhuman: seat(25, "Mail Starter, annual"),
    Obsidian: seat(4, "Sync, annual"),
    ClickUp: seat(7, "Unlimited, annual"),
    Excalidraw: seat(6, "Excalidraw+, annual"),
    FigJam: seat(3, "Professional, annual"),
    Mural: seat(9.99, "Team+, annual"),
    Lucid: seat(7.95, "Lucidspark Individual, annual"),
    "Cal.com": seat(12, "Teams, annual"),
    "Proton Mail": seat(3.99, "Mail Plus, annual"),
    Coda: seat(12, "Pro, annual"),
    Trello: seat(5, "Standard, annual"),
    Asana: seat(10.99, "Starter, annual"),
    Airtable: seat(20, "Team, annual"),
    Miro: seat(8, "Starter, annual"),
    Claude: seat(17, "Pro, annual"),
    Gemini: seat(16.67, "Google AI Pro, annual"),
    Grok: seat(25, "SuperGrok, annual"),

    // per seat, with one price however billed
    Confluence: seat(5.78, "Standard"),
    Jira: seat(8.48, "Standard"),
    Doppler: seat(21, "Team"),
    ChatGPT: seat(20, "Plus, monthly only"),

    // flat, billed annually where offered
    Framer: flat(10, "Basic site, annual"),
    Make: flat(9, "Core, annual"),
    Zapier: flat(19.99, "Professional, annual"),
    n8n: flat(20, "Starter, annual, in euros"),
    Discord: flat(0, "Free"),
    Thunderbird: flat(0, "Free"),
    DeepSeek: flat(0, "Free"),
    WhatsApp: flat(0, "Free"),

    // metered, by each vendor's published meters
    GitHub: metered("Team + Actions", ({ people }) => {
        const minutes = ciMinutesAt(people);

        return SEAT_GITHUB * people + Math.max(0, minutes - ACTIONS_INCLUDED) * ACTIONS_RATE;
    }),
    WorkOS: metered("Free to 1M users", ({ users }) =>
        users <= 1_000_000 ? 0 : Math.ceil((users - 1_000_000) / 1_000_000) * 2_500,
    ),
    Supabase: metered("Pro + usage", ({ users }) => {
        const database = (users * workloadOf("Supabase").perUser.database) / GB;

        return 25 + Math.max(0, users - 100_000) * 0.003_25 + Math.max(0, database - 8) * 0.125;
    }),
    Resend: metered("Pro or Scale", ({ users }) => resendAt(users * EMAILS_PER_USER)),
    Temporal: metered("Actions", ({ users }) =>
        temporalAt(users * ACTIONS_PER_USER * STEPS_PER_ACTION),
    ),
    PostHog: metered("Free to 1M events", ({ users }) => postHogAt(users * EVENTS_PER_USER)),
    Vercel: metered(
        "Pro + CDN",
        ({ people, users }) => 20 * people + cdnAt(users * REQUESTS_PER_USER),
    ),
};

/** The users the figures that count people read a bill for: an app a few months into its launch. */
export const BASE_USERS = 1_000;

/** The bytes a tool keeps, averaged over the month. */
type Kept = { database: number; files: number };

/** The work a tool's job takes: once for the team, per person, per user, and in batch jobs. */
export type Workload = {
    /** What the team keeps once, whatever its size. */
    shared: Kept;
    /** What each person keeps, the hours they have the tool open a workday, and their requests. */
    perPerson: Kept & { hours: number; requests: number };
    /** What an app keeps for each of its users, and the requests each user makes a month. */
    perUser: Kept & { requests: number };
    /** The GB-hours of batch jobs a month, such as CI runs, beside the requests. */
    batch: (scale: Scale) => number;
    /** Whether the work is a floor, since it runs code of the team's own. */
    isAtLeast: boolean;
};

/** A month of work on every meter Destack bills. */
export type Usage = {
    /** The bytes kept in databases. */
    database: number;
    /** The bytes kept in buckets. */
    files: number;
    /** The memory held, in GB-hours. */
    memory: number;
    /** The CPU time spent on requests, in milliseconds. */
    cpu: number;
    /** The requests served. */
    requests: number;
    /** The database rows read. */
    rowsRead: number;
    /** The database rows written. */
    rowsWritten: number;
    /** The bucket reads. */
    fileReads: number;
    /** The bucket writes. */
    fileWrites: number;
};

/** Destack's usage rates in US dollars: per GB-month, per GB-hour, and per million of the rest. */
export type Rates = Readonly<Record<keyof Usage, number>>;

/** Nothing kept. */
const nothing: Kept = { database: 0, files: 0 };

/** Return a workload from the parts of it a tool has. */
function workloadFrom(parts: {
    shared?: Partial<Kept>;
    perPerson?: Partial<Workload["perPerson"]>;
    perUser?: Partial<Workload["perUser"]>;
    batch?: (scale: Scale) => number;
    isAtLeast?: boolean;
}): Workload {
    return {
        shared: { ...nothing, ...parts.shared },
        perPerson: { ...nothing, hours: 0, requests: 0, ...parts.perPerson },
        perUser: { ...nothing, requests: 0, ...parts.perUser },
        batch: parts.batch ?? (() => 0),
        isAtLeast: parts.isAtLeast ?? false,
    };
}

/** Return the workload of a tool each person uses alone, such as their docs or mail. */
function personal(perPerson: Workload["perPerson"]): Workload {
    return workloadFrom({ perPerson });
}

/** Documents: open 2 hours a workday with a request every 5 seconds, and a year of pages. */
const documents = personal({
    hours: 2,
    requests: 1_440,
    // 30 pages a month of 200 blocks at 2 KB with indexes and history, kept a year
    database: 144 * MB,
    // 20 images a month of 400 KB, kept a year
    files: 96 * MB,
});

/** Tasks: open 1.5 hours a workday at 200 requests an hour, and a year of issues. */
const tasks = personal({
    hours: 1.5,
    requests: 300,
    // 100 issues a month of 35 KB with their comments and history, kept a year
    database: 42 * MB,
    // a month's attachments, kept a year
    files: 12 * ATTACHMENTS,
});

/** Chat: open all workday at 120 requests an hour, and a year of messages. */
const chat = personal({
    hours: 8,
    requests: 960,
    // 200 messages a workday at 2 KB with indexes, kept a year
    database: 106 * MB,
    // 2 shared files a workday of 1 MB, kept a year
    files: 528 * MB,
});

/** Mail: open 3 hours a workday at 120 requests an hour, and an 8 GB mailbox. */
const mail = personal({
    hours: 3,
    requests: 360,
    // the mailbox's index, 5% of it
    database: 400 * MB,
    // the bodies and attachments
    files: 7.6 * GB,
});

/** Calendar: open half an hour a workday at 60 requests an hour, and a year of events. */
const calendar = personal({
    hours: 0.5,
    requests: 30,
    // 100 events a month of 2 KB, kept a year
    database: 2.4 * MB,
    files: 0,
});

/** Tables: open an hour a workday at 200 requests an hour, and 5,000 rows with history. */
const tables = personal({ hours: 1, requests: 200, database: 20 * MB, files: 100 * MB });

/** Canvas: open an hour a workday with every cursor move sent, 600 requests an hour. */
const canvas = personal({ hours: 1, requests: 600, database: 30 * MB, files: 30 * MB });

/** Automation: 750 runs a month of 5 steps, and a year of run history at 2 KB a step. */
const automation = personal({ hours: 0.5, requests: 170, database: 90 * MB, files: 0 });

/** Agents: calling into the space an hour a workday at 120 requests an hour. */
const agents = personal({ hours: 1, requests: 120, database: 15 * MB, files: 0 });

/** The work each tool's job takes after a year of use, by tool. */
export const workloads: Readonly<Record<string, Workload>> = {
    // documents
    Notion: documents,
    Docs: documents,
    Word: documents,
    Confluence: documents,
    Obsidian: documents,

    // tasks
    Linear: tasks,
    Jira: tasks,
    Asana: tasks,
    Trello: tasks,
    ClickUp: tasks,

    // chat
    Slack: chat,
    Teams: chat,
    Discord: chat,
    "Google Chat": chat,
    WhatsApp: chat,

    // mail
    Gmail: mail,
    Outlook: mail,
    Superhuman: mail,
    Thunderbird: mail,
    "Proton Mail": mail,

    // calendar
    Calendar: calendar,
    "Outlook Calendar": calendar,
    Calendly: calendar,
    "Cal.com": calendar,

    // tables
    Airtable: tables,
    Sheets: tables,
    Excel: tables,
    Coda: tables,

    // canvas
    Miro: canvas,
    FigJam: canvas,
    Mural: canvas,
    Lucid: canvas,
    Excalidraw: canvas,

    // automation
    Zapier: automation,
    Make: automation,
    n8n: automation,
    "Power Automate": automation,

    // agents
    ChatGPT: agents,
    Claude: agents,
    Gemini: agents,
    DeepSeek: agents,
    Grok: agents,

    // code: a repository, each developer's work, and CI runs
    GitHub: workloadFrom({
        shared: { database: 100 * MB, files: 2.85 * GB },
        perPerson: { hours: 2, requests: 200, database: 50 * MB },
        batch: ({ people }) => (ciMinutesAt(people) / 60) * CI_MEMORY,
    }),

    // a website: its pages, the team's edits, and each visitor's page loads
    Framer: workloadFrom({
        shared: { database: 33 * MB, files: 77 * MB },
        perPerson: { hours: 0.25, requests: 100 },
        perUser: { requests: REQUESTS_PER_USER },
    }),

    // a waitlist app of the team's own code, and each signup's page loads
    Replit: workloadFrom({
        shared: { files: 150 * MB },
        perPerson: { hours: 0.5, requests: 100 },
        perUser: { database: 0.15 * MB, requests: REQUESTS_PER_USER },
        isAtLeast: true,
    }),

    // identity: a 25 KB account with its sessions
    WorkOS: workloadFrom({ perUser: { database: 0.025 * MB, requests: SIGN_IN_REQUESTS } }),
    // secrets: a few hundred keys with their history, read only as instances start
    Doppler: workloadFrom({ shared: { database: 2 * MB } }),
    // data: each user's rows and indexes, 0.4 MB, and 45 KB of uploads
    Supabase: workloadFrom({
        perUser: { database: 0.405 * MB, files: 0.045 * MB, requests: REQUESTS_PER_USER },
    }),
    // messages: a year of 3 emails a month, logged and kept
    Resend: workloadFrom({
        perUser: { database: 0.05 * MB, files: 0.05 * MB, requests: EMAILS_PER_USER },
    }),
    // jobs: a year of workflow history
    Temporal: workloadFrom({
        perUser: { database: 0.2 * MB, requests: ACTIONS_PER_USER * STEPS_PER_ACTION },
    }),
    // analytics: a year of 50 events a month at 1.5 KB
    PostHog: workloadFrom({ perUser: { database: 0.9 * MB, requests: EVENTS_PER_USER } }),
    // hosting: the app's build and assets, and each user's page loads
    Vercel: workloadFrom({
        shared: { database: 40 * MB, files: 1.96 * GB },
        perUser: { requests: REQUESTS_PER_USER },
        isAtLeast: true,
    }),
};

/** Return the CI minutes a repository runs a month for a team of developers. */
function ciMinutesAt(people: number): number {
    return CI_MINUTES + (people - 1) * CI_MINUTES_PER_DEVELOPER;
}

/** Return the work a workload takes for a team and its audience, on every meter. */
export function usageAt(workload: Workload, scale: Scale): Usage {
    // keep the team's bytes, each person's and each user's
    const kept = (key: keyof Kept) =>
        workload.shared[key] +
        workload.perPerson[key] * scale.people +
        workload.perUser[key] * scale.users;

    // count each person's requests over their workdays and each user's over the month
    const personRequests = workload.perPerson.requests * WORKDAYS * scale.people;
    const userRequests = workload.perUser.requests * scale.users;
    const requests = personRequests + userRequests;

    // hold instances awake for every open hour and every user request, none overlapping
    const hours = workload.perPerson.hours * WORKDAYS * scale.people;
    const awake = hours + (userRequests * AWAKE_SECONDS) / SECONDS_PER_HOUR;
    const instances = instancesAt(scale.people, userRequests);
    const memory = INSTANCE_MEMORY * Math.min(awake, instances * HOURS_PER_MONTH);

    // write every file again each month, and read each write ten times
    const files = kept("files");
    const fileWrites = files / FILE_SIZE;

    return {
        database: kept("database"),
        files,
        memory: memory + workload.batch(scale),
        cpu: requests * CPU_PER_REQUEST,
        requests,
        rowsRead: requests * ROWS_READ_PER_REQUEST,
        rowsWritten: requests * ROWS_WRITTEN_PER_REQUEST,
        fileReads: fileWrites * FILE_READS_PER_WRITE,
        fileWrites,
    };
}

/** Return the instances a team and its users keep busy: one per 25 people or per peak load. */
function instancesAt(people: number, userRequests: number): number {
    // count an instance per 25 people and per 50 requests a second at the busiest hour
    const peak = (PEAK_FACTOR * userRequests) / (HOURS_PER_MONTH * SECONDS_PER_HOUR);
    const byPeople = Math.ceil(people / PEOPLE_PER_INSTANCE);
    const byLoad = Math.ceil(peak / REQUESTS_PER_SECOND);

    return Math.max(1, byPeople, byLoad);
}

/** Return the sum of workloads, such as the tools a row puts together. */
export function sumOf(tools: readonly Workload[]): Workload {
    return {
        shared: {
            database: totalOf(tools.map((each) => each.shared.database)),
            files: totalOf(tools.map((each) => each.shared.files)),
        },
        perPerson: {
            database: totalOf(tools.map((each) => each.perPerson.database)),
            files: totalOf(tools.map((each) => each.perPerson.files)),
            hours: totalOf(tools.map((each) => each.perPerson.hours)),
            requests: totalOf(tools.map((each) => each.perPerson.requests)),
        },
        perUser: {
            database: totalOf(tools.map((each) => each.perUser.database)),
            files: totalOf(tools.map((each) => each.perUser.files)),
            requests: totalOf(tools.map((each) => each.perUser.requests)),
        },
        batch: (scale) => totalOf(tools.map((each) => each.batch(scale))),
        isAtLeast: tools.some((each) => each.isAtLeast),
    };
}

/** Return the sum of numbers. */
function totalOf(values: readonly number[]): number {
    return values.reduce((sum, value) => sum + value, 0);
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

/** Return what Temporal Cloud charges for a month's actions: $50 a million to 5M, then $25. */
function temporalAt(actions: number): number {
    const first = Math.min(actions, 5_000_000);
    const rest = Math.max(0, actions - 5_000_000);

    return first * 0.000_05 + rest * 0.000_025;
}

/** Return what Vercel's flat rate CDN charges for a month's requests: free to 1M, then by tier. */
function cdnAt(requests: number): number {
    // read the tiers, each with the requests it covers
    const tiers: readonly (readonly [requests: number, monthly: number])[] = [
        [1_000_000, 0],
        [10_000_000, 20],
        [50_000_000, 100],
        [150_000_000, 300],
    ];

    // pick the smallest tier that covers the requests
    const tier = tiers.find(([included]) => requests <= included);

    // take the top tier again for every 150M requests past it
    return tier === undefined ? 300 * Math.ceil(requests / 150_000_000) : tier[1];
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
        [250_000_000, 0.000_015],
        [Number.POSITIVE_INFINITY, 0.000_009],
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

/** Return what a month's work costs at Destack's unit prices, on every meter it bills. */
export function usageCostOf(usage: Usage, rates: Rates): number {
    // price the bytes kept by the GB-month, and the memory by the GB-hour
    const kept = (usage.database / GB) * rates.database + (usage.files / GB) * rates.files;
    const memory = usage.memory * rates.memory;

    // price every counted meter by the million
    const counted =
        usage.cpu * rates.cpu +
        usage.requests * rates.requests +
        usage.rowsRead * rates.rowsRead +
        usage.rowsWritten * rates.rowsWritten +
        usage.fileReads * rates.fileReads +
        usage.fileWrites * rates.fileWrites;

    return kept + memory + counted / MILLION;
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
