import { priceOf } from "../figure/price";
import type { Item } from "../figure/item";

/** The ten services a docs app runs on, numbered as the ledger rows. */
export const services: readonly { stacked: Item; destacked: Item }[] = [
    {
        stacked: {
            label: "Identity",
            mark: { logo: "workos.png" },
            name: "WorkOS",
            note: "sign-in and roles in a vendor of their own",
            price: priceOf("WorkOS"),
        },
        destacked: {
            label: "Identity",
            mark: { app: "accounts" },
            name: "Accounts",
            note: "one sign-in and one permission model",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Secrets",
            mark: { logo: "doppler.png" },
            name: "Doppler",
            note: "keys pasted into every service",
            price: priceOf("Doppler"),
        },
        destacked: {
            label: "Secrets",
            mark: { app: "vault" },
            name: "Vault",
            note: "keys kept in one vault",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Database",
            mark: { logo: "supabase.svg" },
            name: "Supabase",
            note: "a second backend to keep in step",
            price: priceOf("Supabase"),
        },
        destacked: {
            label: "Database",
            mark: { app: "storage" },
            name: "Database",
            note: "one backend, always in step",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Files",
            mark: { logo: "dropbox.svg" },
            name: "Dropbox",
            note: "files kept in their storage",
            price: priceOf("Dropbox"),
        },
        destacked: {
            label: "Files",
            mark: { app: "bucket" },
            name: "Files",
            note: "files kept in your own storage",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Search",
            mark: { logo: "elastic.svg" },
            name: "Elastic",
            note: "a second index to keep in sync",
            price: priceOf("Elastic"),
        },
        destacked: {
            label: "Search",
            mark: { app: "search" },
            name: "Search",
            note: "one index, always in sync",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Messaging",
            mark: { logo: "resend.svg" },
            name: "Resend",
            note: "an extra SDK just for email",
            price: priceOf("Resend"),
        },
        destacked: {
            label: "Messaging",
            mark: { app: "notify" },
            name: "Notifications",
            note: "one notification system for all apps",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Jobs",
            mark: { logo: "temporal.png" },
            name: "Temporal",
            note: "a worker fleet beside the app",
            price: priceOf("Temporal"),
        },
        destacked: {
            label: "Jobs",
            mark: { app: "sync" },
            name: "Workflows",
            note: "workflows running inside the app",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Analytics",
            mark: { logo: "posthog.svg" },
            name: "PostHog",
            note: "events sent to their cloud",
            price: priceOf("PostHog"),
        },
        destacked: {
            label: "Analytics",
            mark: { app: "telemetry" },
            name: "Telemetry",
            note: "events and errors in your space",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Hosting",
            mark: { logo: "vercel.png" },
            name: "Vercel",
            note: "runs only in their cloud",
            price: priceOf("Vercel"),
        },
        destacked: {
            label: "Hosting",
            mark: { app: "hosts" },
            name: "Hosting",
            note: "runs on your laptop, server or cloud",
            price: { kind: "included" },
        },
    },
    {
        stacked: {
            label: "Code",
            mark: { logo: "github.png" },
            name: "GitHub",
            note: "a pipeline of its own",
            price: priceOf("GitHub"),
        },
        destacked: {
            label: "Code",
            mark: { app: "source" },
            name: "Forge",
            note: "one pipeline for all apps",
            price: { kind: "included" },
        },
    },
];
