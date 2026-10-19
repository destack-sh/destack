import type { Category } from "../tool/tool";

/** The bytes in a megabyte. */
const MB = 1_000_000;

/** The bytes in a gigabyte. */
const GB = 1_000_000_000;

/** The CI minutes a first developer runs a month: 20 runs a day of 10 minutes over 22 workdays. */
const CI_MINUTES = 4_400;

/** The CI minutes each further developer adds a month: 7 runs a day of 10 minutes, 22 days. */
const CI_MINUTES_PER_DEVELOPER = 1_540;

/** The memory a CI run holds in GB, its GB-hours above a 4 vCPU, 8 GB machine's hourly price. */
const CI_MEMORY = 2;

/** The build minutes a person's deploys take a month: five deploys a workday of three minutes. */
const BUILD_MINUTES = 330;

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

/** The sessions each user records a month: every session of the two a week. */
const RECORDINGS_PER_USER = 8;

/** The size of a session recording: five minutes of compressed page changes at 1 KB a second. */
const RECORDING_SIZE = 0.3 * MB;

/** WorkOS's price for a custom sign-in domain a month in US dollars. */
const CUSTOM_DOMAIN_WORKOS = 99;

/** The workflow actions each user starts a month: a few jobs of about five steps. */
const ACTIONS_PER_USER = 20;

/** The requests a page load makes. */
const REQUESTS_PER_PAGE = 7;

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

/** Auth0 B2C Essentials' price for a monthly active user, its $35 for 500 MAU tier spread out. */
const AUTH0_MAU = 0.07;

/** Clerk Pro's price for each monthly active user past its first 10,000. */
const CLERK_MAU = 0.02;

/** The monthly active users Clerk Pro includes. */
const CLERK_INCLUDED = 10_000;

/** Neon Launch's price for a compute unit hour, 1 vCPU and 4 GB, in US dollars. */
const NEON_COMPUTE_HOUR = 0.106;

/** Neon's price for a GB of storage a month in US dollars. */
const NEON_STORAGE = 0.35;

/** Firestore's price for 100,000 document reads in US dollars. */
const FIRESTORE_READS = 0.06;

/** Firestore's price for 100,000 document writes in US dollars. */
const FIRESTORE_WRITES = 0.18;

/** Firestore's price for a GB of storage a month in US dollars. */
const FIRESTORE_STORAGE = 0.18;

/** The document reads Firestore gives free a month: 50,000 a day. */
const FIRESTORE_FREE_READS = 1_500_000;

/** The document writes Firestore gives free a month: 20,000 a day. */
const FIRESTORE_FREE_WRITES = 600_000;

/** The workflow steps Upstash gives free a month: 1,000 a day. */
const UPSTASH_FREE_STEPS = 30_000;

/** The client errors each user meets a month: about one in every four sessions. */
const ERRORS_PER_USER = 2;

/** Datadog's price for a host a month in US dollars: Infrastructure Pro and APM, annual. */
const DATADOG_HOST = 46;

/** Datadog's price for 1,000 sessions in US dollars: RUM Measure and Session Replay, annual. */
const DATADOG_SESSIONS = 2.65;

/** The credits Netlify Pro includes a month. */
const NETLIFY_CREDITS = 3_000;

/** The credits a production deploy takes on Netlify. */
const NETLIFY_DEPLOY_CREDITS = 15;

/** The deploys a person ships a month: five a workday. */
const DEPLOYS_PER_PERSON = 110;

/** The megabytes each user's page loads transfer a month: forty loads of 1 MB. */
const TRANSFER_PER_USER = 40;

/** The credits a GB of bandwidth takes on Netlify. */
const NETLIFY_BANDWIDTH_CREDITS = 10;

/** The credits 10,000 requests take on Netlify. */
const NETLIFY_REQUEST_CREDITS = 2;

/** The minutes of CI GitLab Premium includes a month. */
const GITLAB_MINUTES = 10_000;

/** The build minutes Bitbucket Standard includes a month. */
const BITBUCKET_MINUTES = 2_500;

/** A volume tier: the amount it prices up to, and its rate for each unit. */
type Tier = readonly [upTo: number, rate: number];

/** PostHog's tiers for analytics events a month, a free million first. */
const POSTHOG_EVENTS: readonly Tier[] = [
    [1_000_000, 0],
    [2_000_000, 0.000_05],
    [15_000_000, 0.000_034_3],
    [50_000_000, 0.000_029_5],
    [100_000_000, 0.000_021_8],
    [250_000_000, 0.000_015],
    [Number.POSITIVE_INFINITY, 0.000_009],
];

/** PostHog's tiers for web session recordings a month, 5,000 free first. */
const POSTHOG_RECORDINGS: readonly Tier[] = [
    [5_000, 0],
    [15_000, 0.005],
    [50_000, 0.003_5],
    [150_000, 0.002],
    [500_000, 0.001_7],
    [Number.POSITIVE_INFINITY, 0.001_5],
];

/** The size of a team and of its app's audience a bill is read for. */
export type Scale = { people: number; users: number };

/** What an item costs a month. */
export type Price =
    | { kind: "fee"; monthlyAt: (scale: Scale) => number; basis: string; listed: string }
    | { kind: "bundled"; basis: string }
    | { kind: "included" };

/** Return a monthly fee charged for every person on the plan. */
function seat(monthly: number, basis: string): Price {
    const listed = `$${amountOf(monthly)}/seat/mo`;

    return { kind: "fee", monthlyAt: (scale) => monthly * scale.people, basis, listed };
}

/** Return a monthly fee charged once, however many people use it. */
export function flat(monthly: number, basis: string): Price {
    return { kind: "fee", monthlyAt: () => monthly, basis, listed: `$${amountOf(monthly)}/mo` };
}

/** Return a fee that follows the team and its audience, by a vendor's published meters. */
function metered(basis: string, listed: string, monthlyAt: (scale: Scale) => number): Price {
    return { kind: "fee", monthlyAt, basis, listed };
}

/** Return a list price's amount as a vendor states it: whole dollars, or to the cent. */
function amountOf(monthly: number): string {
    return Number.isInteger(monthly) ? String(monthly) : monthly.toFixed(2);
}

/** The list prices on 2026-10-17 of each tool's least plan with private spaces and guests. */
export const prices: Readonly<Record<string, Price>> = {
    // per seat, billed annually
    Notion: seat(20, "Business, annual"),
    Linear: seat(16, "Business, annual"),
    Slack: seat(7.25, "Pro, annual"),
    Calendly: seat(16, "Teams, annual"),
    "Google Workspace": seat(14, "Business Standard, annual"),
    "Microsoft 365": seat(7, "Business Basic, annual"),
    Superhuman: seat(33, "Business, annual"),
    Obsidian: seat(4, "Sync, annual"),
    ClickUp: seat(7, "Unlimited, annual"),
    Excalidraw: seat(6, "Excalidraw+, annual"),
    FigJam: seat(3, "Professional, annual"),
    Mural: seat(9.99, "Team+, annual"),
    Lucid: seat(10, "Lucidspark Team, annual"),
    "Cal.com": seat(12, "Teams, annual"),
    "Proton Mail": seat(9.99, "Mail Professional, annual"),
    Coda: seat(30, "Team, annual"),
    Trello: seat(5, "Standard, annual"),
    Asana: seat(10.99, "Starter, annual"),
    Airtable: seat(20, "Team, annual"),
    Miro: seat(16, "Business, annual"),
    Dropbox: seat(15, "Standard, annual"),
    Figma: seat(16, "Professional, annual"),
    Granola: seat(14, "Business, monthly"),

    // per seat, with one price however billed
    Confluence: seat(5.78, "Standard"),
    Jira: seat(8.48, "Standard"),
    Doppler: seat(21, "Team"),

    // flat, billed annually where offered
    n8n: flat(20, "Starter, annual, in euros"),
    Typeform: flat(56, "Plus, annual"),
    Discord: flat(0, "Free"),
    Thunderbird: flat(0, "Free"),
    WhatsApp: flat(0, "Free"),

    // one person's plan, or the team plan from a second person on, billed annually
    Framer: metered(
        "Pro site + editors, annual",
        "$30/site/mo + $20/editor/mo",
        ({ people }) => 30 + 20 * (people - 1),
    ),
    Replit: metered("Core, or Pro, annual", "$20/mo, or $100/mo for a team", ({ people }) =>
        people === 1 ? 20 : 100,
    ),
    Lovable: metered("Pro, monthly", "$25/mo, shared by the workspace", () => 25),
    Bolt: metered("Pro, or Teams", "$25/mo, or $30/member/mo", ({ people }) =>
        people === 1 ? 25 : 30 * people,
    ),
    v0: metered(
        "Team + Vercel Pro",
        "$30/seat/mo + $20/seat/mo hosting",
        ({ people }) => 50 * people,
    ),
    Make: metered("Core, or Teams, annual", "$9/mo, or $29/mo for a team", ({ people }) =>
        people === 1 ? 9 : 29,
    ),
    Zapier: metered(
        "Professional, or Team, annual",
        "$19.99/mo, or $69/mo for a team",
        ({ people }) => (people === 1 ? 19.99 : 69),
    ),

    // metered, by each vendor's published meters
    GitHub: metered("Team + Actions", "$4/seat/mo + Actions", ({ people }) => {
        const minutes = ciMinutesAt(people);

        return SEAT_GITHUB * people + Math.max(0, minutes - ACTIONS_INCLUDED) * ACTIONS_RATE;
    }),
    WorkOS: metered("AuthKit + custom domain", "$99/mo + $2,500/1M users", ({ users }) => {
        const extra = Math.ceil(Math.max(0, users - 1_000_000) / 1_000_000);

        return CUSTOM_DOMAIN_WORKOS + extra * 2_500;
    }),
    Supabase: metered("Pro + Small compute", "$30/mo + usage", ({ users }) => {
        const database = (users * workloadOf("Supabase").perUser.database) / GB;

        return 30 + Math.max(0, users - 100_000) * 0.003_25 + Math.max(0, database - 8) * 0.125;
    }),
    Resend: metered("Pro or Scale", "from $20/mo", ({ users }) =>
        resendAt(users * EMAILS_PER_USER),
    ),
    Temporal: metered("Essentials", "from $100/mo", ({ users }) =>
        temporalAt(users * ACTIONS_PER_USER * STEPS_PER_ACTION),
    ),
    PostHog: metered("Analytics + replay", "per usage, free to 1M events", ({ users }) => {
        const events = tieredAt(users * EVENTS_PER_USER, POSTHOG_EVENTS);
        const recordings = tieredAt(users * RECORDINGS_PER_USER, POSTHOG_RECORDINGS);

        return events + recordings;
    }),
    Vercel: metered(
        "Pro + CDN",
        "$20/seat/mo + usage",
        ({ people, users }) => 20 * people + cdnAt(users * REQUESTS_PER_USER),
    ),

    // the alternatives fig. 3 turns through, priced the same way
    Clerk: metered(
        "Pro",
        "$25/mo + $0.02/MAU past 10K",
        ({ users }) => 25 + Math.max(0, users - CLERK_INCLUDED) * CLERK_MAU,
    ),
    Auth0: metered("B2C Essentials", "from $35/mo for 500 MAU", ({ users }) =>
        Math.max(35, users * AUTH0_MAU),
    ),
    "1Password": seat(7.99, "Business, annual"),
    Convex: metered("Professional", "$25/developer/mo + usage", ({ people }) => 25 * people),
    Neon: metered("Launch", "$0.106/CU-hour + $0.35/GB-month", ({ users }) => neonAt(users)),
    Firebase: metered("Blaze", "$0.06/100K reads + $0.18/100K writes", ({ users }) =>
        firestoreAt(users),
    ),
    Mailgun: metered("Basic or Foundation", "from $15/mo", ({ users }) =>
        mailgunAt(users * EMAILS_PER_USER),
    ),
    Upstash: metered(
        "Workflow, pay as you go",
        "$1/100K steps",
        ({ users }) =>
            Math.max(0, users * ACTIONS_PER_USER * STEPS_PER_ACTION - UPSTASH_FREE_STEPS) / 100_000,
    ),
    Datadog: metered(
        "Infrastructure Pro + APM + RUM",
        "$46/host/mo + $2.65/1K sessions",
        ({ users }) =>
            DATADOG_HOST * hostsAt(users) +
            ((users * RECORDINGS_PER_USER) / 1000) * DATADOG_SESSIONS,
    ),
    Sentry: metered(
        "Team, annual",
        "$26/mo + usage",
        ({ users }) => 26 + Math.max(0, users * ERRORS_PER_USER - 50_000) * 0.000_29,
    ),
    Netlify: metered("Pro + credits", "$20/mo + $10/1,500 credits", ({ people, users }) =>
        netlifyAt(people, users),
    ),
    Render: metered(
        "Professional + Standard",
        "$19/seat/mo + $25/instance",
        ({ people, users }) => 19 * people + 25 * hostsAt(users),
    ),
    GitLab: metered(
        "Premium + compute",
        "$29/seat/mo + minutes",
        ({ people }) => 29 * people + minutePacksAt(ciMinutesAt(people) - GITLAB_MINUTES),
    ),
    Bitbucket: metered(
        "Standard + build minutes",
        "$3.65/seat/mo + minutes",
        ({ people }) => 3.65 * people + minutePacksAt(ciMinutesAt(people) - BITBUCKET_MINUTES),
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

/** Calendar: open half an hour a workday, and invitees loading 2 booking pages a workday. */
const calendar = personal({
    hours: 0.5,
    requests: 30 + 2 * REQUESTS_PER_PAGE,
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

/** Connection: a personal app synced every 15 minutes, 10 seconds a sync, and a year of rows. */
const connection = personal({ hours: 0.35, requests: 131, database: 10 * MB, files: 0 });

/** Builder: a waitlist app of the team's own code, and each signup's page loads. */
const builder = workloadFrom({
    shared: { files: 150 * MB },
    perPerson: { hours: 0.5, requests: 100 },
    perUser: { database: 0.15 * MB, requests: REQUESTS_PER_USER },
    isAtLeast: true,
});

/** The work each job takes after a year of use, whichever tool does it. */
export const jobWorkloads: Readonly<Record<Category, Workload>> = {
    documents,
    tables,
    tasks,
    calendar,
    chat,
    mail,
    canvas,
    workflows: automation,
    builder,
};

/** The work each tool outside the jobs takes after a year of use, by tool. */
export const workloads: Readonly<Record<string, Workload>> = {
    // personal apps connected to your space
    Strava: connection,
    Monzo: connection,
    Goodreads: connection,
    Steam: connection,
    Letterboxd: connection,

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
    // analytics: a year of 50 events a month at 1.5 KB, and a month of session recordings
    PostHog: workloadFrom({
        perUser: {
            database: 0.9 * MB,
            files: RECORDINGS_PER_USER * RECORDING_SIZE,
            requests: EVENTS_PER_USER,
        },
    }),
    // the alternatives fig. 3 turns through, each doing the same job as its row's first
    Clerk: workloadFrom({ perUser: { database: 0.025 * MB, requests: SIGN_IN_REQUESTS } }),
    Auth0: workloadFrom({ perUser: { database: 0.025 * MB, requests: SIGN_IN_REQUESTS } }),
    "1Password": workloadFrom({ shared: { database: 2 * MB } }),
    Convex: workloadFrom({
        perUser: { database: 0.405 * MB, files: 0.045 * MB, requests: REQUESTS_PER_USER },
    }),
    Neon: workloadFrom({
        perUser: { database: 0.405 * MB, files: 0.045 * MB, requests: REQUESTS_PER_USER },
    }),
    Firebase: workloadFrom({
        perUser: { database: 0.405 * MB, files: 0.045 * MB, requests: REQUESTS_PER_USER },
    }),
    Mailgun: workloadFrom({
        perUser: { database: 0.05 * MB, files: 0.05 * MB, requests: EMAILS_PER_USER },
    }),
    Upstash: workloadFrom({
        perUser: { database: 0.2 * MB, requests: ACTIONS_PER_USER * STEPS_PER_ACTION },
    }),
    Datadog: workloadFrom({
        perUser: {
            database: 0.9 * MB,
            files: RECORDINGS_PER_USER * RECORDING_SIZE,
            requests: EVENTS_PER_USER,
        },
    }),
    Sentry: workloadFrom({
        perUser: {
            database: 0.9 * MB,
            files: RECORDINGS_PER_USER * RECORDING_SIZE,
            requests: EVENTS_PER_USER,
        },
    }),
    Netlify: workloadFrom({
        shared: { database: 40 * MB, files: 1.96 * GB },
        perUser: { requests: REQUESTS_PER_USER },
        batch: ({ people }) => ((BUILD_MINUTES * people) / 60) * CI_MEMORY,
        isAtLeast: true,
    }),
    Render: workloadFrom({
        shared: { database: 40 * MB, files: 1.96 * GB },
        perUser: { requests: REQUESTS_PER_USER },
        batch: ({ people }) => ((BUILD_MINUTES * people) / 60) * CI_MEMORY,
        isAtLeast: true,
    }),
    GitLab: workloadFrom({
        shared: { database: 100 * MB, files: 2.85 * GB },
        perPerson: { hours: 2, requests: 200, database: 50 * MB },
        batch: ({ people }) => (ciMinutesAt(people) / 60) * CI_MEMORY,
    }),
    Bitbucket: workloadFrom({
        shared: { database: 100 * MB, files: 2.85 * GB },
        perPerson: { hours: 2, requests: 200, database: 50 * MB },
        batch: ({ people }) => (ciMinutesAt(people) / 60) * CI_MEMORY,
    }),

    // hosting: the app's builds and assets, and each user's page loads
    Vercel: workloadFrom({
        shared: { database: 40 * MB, files: 1.96 * GB },
        perUser: { requests: REQUESTS_PER_USER },
        batch: ({ people }) => ((BUILD_MINUTES * people) / 60) * CI_MEMORY,
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

/** Return the hosts an app's peak load takes, each serving 50 requests a second. */
function hostsAt(users: number): number {
    const peak = (users * REQUESTS_PER_USER * PEAK_FACTOR) / (HOURS_PER_MONTH * SECONDS_PER_HOUR);

    return Math.max(1, Math.ceil(peak / REQUESTS_PER_SECOND));
}

/** Return what Neon Launch charges a month: a compute sized to the audience, and storage. */
function neonAt(users: number): number {
    const units = users <= 10_000 ? 0.25 : users <= 50_000 ? 0.5 : 1;
    const storage = (users * workloadOf("Supabase").perUser.database) / GB;

    return units * HOURS_PER_MONTH * NEON_COMPUTE_HOUR + storage * NEON_STORAGE;
}

/** Return what Firestore charges a month for reads, writes and storage past its free quota. */
function firestoreAt(users: number): number {
    // count the reads, writes and stored GB past what Firestore gives free
    const requests = users * REQUESTS_PER_USER;
    const reads = Math.max(0, requests * ROWS_READ_PER_REQUEST - FIRESTORE_FREE_READS);
    const writes = Math.max(0, requests * ROWS_WRITTEN_PER_REQUEST - FIRESTORE_FREE_WRITES);
    const storage = Math.max(0, (users * workloadOf("Supabase").perUser.database) / GB - 1);

    // price each meter
    return (
        (reads / 100_000) * FIRESTORE_READS +
        (writes / 100_000) * FIRESTORE_WRITES +
        storage * FIRESTORE_STORAGE
    );
}

/** Return what Mailgun charges for a month's emails: Basic, then Foundation with overage. */
function mailgunAt(emails: number): number {
    return emails <= 10_000 ? 15 : 35 + Math.max(0, emails - 50_000) * 0.0013;
}

/** Return what Netlify Pro charges a month: its fee, and credit packs past its 3,000 credits. */
function netlifyAt(people: number, users: number): number {
    // count the credits deploys, bandwidth and requests take past the plan's
    const deploys = people * DEPLOYS_PER_PERSON * NETLIFY_DEPLOY_CREDITS;
    const bandwidth = ((users * TRANSFER_PER_USER) / 1000) * NETLIFY_BANDWIDTH_CREDITS;
    const requests = ((users * REQUESTS_PER_USER) / 10_000) * NETLIFY_REQUEST_CREDITS;
    const extra = Math.max(0, deploys + bandwidth + requests - NETLIFY_CREDITS);

    // buy the extra credits in packs of 1,500
    return 20 + Math.ceil(extra / 1500) * 10;
}

/** Return what packs of 1,000 CI minutes cost past a plan's included minutes, $10 each. */
function minutePacksAt(minutes: number): number {
    return Math.ceil(Math.max(0, minutes) / 1000) * 10;
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

/** Return what Temporal Cloud Essentials charges a month: $100 for 1M actions, then by use. */
function temporalAt(actions: number): number {
    const middle = Math.min(Math.max(0, actions - 1_000_000), 4_000_000);
    const rest = Math.max(0, actions - 5_000_000);

    return 100 + middle * 0.000_05 + rest * 0.000_025;
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

/** Return what volume tiers charge for an amount, each tier's share at its own rate. */
function tieredAt(amount: number, tiers: readonly Tier[]): number {
    // add each tier's share of the amount at its rate
    let from = 0;
    let cost = 0;
    for (const [upTo, rate] of tiers) {
        cost += Math.max(0, Math.min(amount, upTo) - from) * rate;
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

/** Return a subscription's list price as its vendor states it, or refuse one without a fee. */
export function listedOf(subscription: string): string {
    const price = priceOf(subscription);
    if (price.kind !== "fee") {
        throw new Error(`no list price for ${subscription}`);
    }

    return price.listed;
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
