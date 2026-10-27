import { present } from "@destack/schema";

import { priceOf } from "../figure/price";
import type { Entry, Item } from "../figure/item";
import type { OptionMark } from "../figure/picker";

/** What an app rents a service for. */
export type ServiceCategory =
    | "identity"
    | "secrets"
    | "database"
    | "messaging"
    | "jobs"
    | "analytics"
    | "observability"
    | "hosting"
    | "code";

/** A service an app rents: its name, which is also its plan's, what it is for, and its mark. */
export type Service = { name: string; category: ServiceCategory; mark: OptionMark };

/** Every service fig. 2 rents, grouped by category, the most common one first. */
export const vendors: readonly Service[] = [
    // identity
    { name: "WorkOS", category: "identity", mark: { logo: "workos.png" } },
    { name: "Clerk", category: "identity", mark: { icon: "clerk", tint: "#6c47ff" } },
    { name: "Auth0", category: "identity", mark: { icon: "auth0", tint: "#eb5424" } },

    // secrets
    { name: "Doppler", category: "secrets", mark: { logo: "doppler.png" } },
    { name: "1Password", category: "secrets", mark: { icon: "1password", tint: "#0094f5" } },

    // database
    { name: "Supabase", category: "database", mark: { logo: "supabase.svg" } },
    { name: "Convex", category: "database", mark: { icon: "convex", tint: "#ee342f" } },
    { name: "Neon", category: "database", mark: { icon: "neon", tint: "#00a87e" } },
    { name: "Firebase", category: "database", mark: { icon: "firebase", tint: "#dd2c00" } },

    // messaging
    { name: "Resend", category: "messaging", mark: { logo: "resend.svg" } },
    { name: "Mailgun", category: "messaging", mark: { icon: "mailgun", tint: "#f06b66" } },

    // jobs
    { name: "Temporal", category: "jobs", mark: { logo: "temporal.png" } },
    { name: "Upstash", category: "jobs", mark: { icon: "upstash", tint: "#00c98d" } },

    // analytics
    { name: "PostHog", category: "analytics", mark: { logo: "posthog.svg" } },

    // observability
    { name: "Sentry", category: "observability", mark: { icon: "sentry", tint: "#362d59" } },
    { name: "Datadog", category: "observability", mark: { icon: "datadog", tint: "#632ca6" } },

    // hosting
    { name: "Vercel", category: "hosting", mark: { logo: "vercel.png" } },
    { name: "Netlify", category: "hosting", mark: { icon: "netlify", tint: "#00ad9f" } },
    { name: "Render", category: "hosting", mark: { icon: "render", tint: "#000000" } },

    // code
    { name: "GitHub", category: "code", mark: { logo: "github.png" } },
    { name: "GitLab", category: "code", mark: { icon: "gitlab", tint: "#fc6d26" } },
    { name: "Bitbucket", category: "code", mark: { icon: "bitbucket", tint: "#0052cc" } },
];

/** One row of fig. 2: what the app rents, what renting means, and the engine's part for it. */
type Slot = {
    /** What the app rents a service for. */
    category: ServiceCategory;
    /** The engine's part that does the job. */
    destacked: Item;
};

/** The nine rows a docs app runs on, numbered as the ledger rows. */
const slots: readonly Slot[] = [
    {
        category: "identity",
        destacked: {
            label: "Identity",
            mark: { app: "accounts" },
            name: "Accounts",
            price: { kind: "included" },
        },
    },
    {
        category: "secrets",
        destacked: {
            label: "Secrets",
            mark: { app: "vault" },
            name: "Vault",
            price: { kind: "included" },
        },
    },
    {
        category: "database",
        destacked: {
            label: "Database",
            mark: { app: "storage" },
            name: "Database",
            price: { kind: "included" },
        },
    },
    {
        category: "messaging",
        destacked: {
            label: "Messaging",
            mark: { app: "notify" },
            name: "Notifications",
            price: { kind: "included" },
        },
    },
    {
        category: "jobs",
        destacked: {
            label: "Jobs",
            mark: { app: "sync" },
            name: "Workflows",
            price: { kind: "included" },
        },
    },
    {
        category: "analytics",
        destacked: {
            label: "Analytics",
            mark: { app: "telemetry" },
            name: "Telemetry",
            price: { kind: "included" },
        },
    },
    {
        category: "observability",
        destacked: {
            label: "Observability",
            mark: { app: "audit" },
            name: "Observability",
            price: { kind: "included" },
        },
    },
    {
        category: "hosting",
        destacked: {
            label: "Hosting",
            mark: { app: "hosts" },
            name: "Hosting",
            price: { kind: "included" },
        },
    },
    {
        category: "code",
        destacked: {
            label: "Code",
            mark: { app: "source" },
            name: "Forge",
            price: { kind: "included" },
        },
    },
];

/** The rows of fig. 2, as many as the services the app rents. */
export const serviceCount = slots.length;

/** The label of each row, as the x-ray tags the parts. */
export const serviceLabels: readonly string[] = slots.map((slot) => slot.destacked.label);

/** The engine's part on each row. */
export const serviceParts: readonly Item[] = slots.map((slot) => slot.destacked);

/** The service each category rents. */
export type ServiceLineup = (category: ServiceCategory) => Service;

/** Return the services of a category, the most common one first. */
export function vendorsOf(category: ServiceCategory): readonly Service[] {
    return vendors.filter((vendor) => vendor.category === category);
}

/** Return a service by its name. */
export function vendorOf(name: string): Service {
    return present(
        vendors.find((vendor) => vendor.name === name),
        `service ${name}`,
    );
}

/** Return the index of the row that rents for a category. */
export function serviceIndexOf(category: ServiceCategory): number {
    return slots.findIndex((slot) => slot.category === category);
}

/** Return the category the row at an index rents for. */
export function categoryOf(index: number): ServiceCategory {
    return present(slots[index], "service row").category;
}

/** Return the service each row rents in the lineup. */
export function rentedWith(lineup: ServiceLineup): readonly Service[] {
    return slots.map((slot) => lineup(slot.category));
}

/** Return each row as a bill entry, rented from its service in the lineup. */
export function entriesWith(lineup: ServiceLineup): readonly Entry[] {
    return slots.map((slot) => {
        const vendor = lineup(slot.category);

        return {
            stacked: {
                label: slot.destacked.label,
                mark: vendor.mark,
                name: vendor.name,
                price: priceOf(vendor.name),
            },
            destacked: slot.destacked,
        };
    });
}
