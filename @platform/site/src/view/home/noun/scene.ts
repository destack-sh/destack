import { artifactAddressOf } from "../command/artifact";

import { type Connector, SHELL } from "../command/chat";
import { type Item, type Lead, Scene } from "../command/scene";
import type { Maker } from "./maker";
import { commitments } from "../verb/corner";

/** The Monday page's rows, as its preview shows them: what you said, to whom, and its state. */
const leads: readonly Lead[] = commitments
    .filter(([, , , state]) => state !== "kept")
    .map(([person, what, , state]) => [what, person, state === "open" ? "Open" : "Waiting"]);

/** What you ask your agent to build. */
const ask: Item = {
    kind: "ask",
    text: "make me a monday page. what I promised people in emails and haven't done yet, what's due this week, and the gaps in my calendar to actually do it",
};

/** What you ask once the app works. */
const online: Item = { kind: "ask", text: "ok nice. put it online, I want it on my phone" };

/** Your agent building the app in your space, every service it needs included. */
const build: Item = {
    kind: "call",
    server: "destack",
    running: "Building in your space",
    title: "Built monday",
    request: 'client.app.create({ name: "monday", reads: ["mail", "chat", "tasks", "calendar"] })',
    response: "list · calendar · person card · sign-in · tables · jobs · hosting",
    isFailed: false,
    isExpanded: false,
};

/** The conversation in your space: the app built and live in one go. */
export const destackedScene = Scene.of([
    ask,
    build,
    {
        kind: "app",
        title: "Monday",
        badge: "Live",
        rows: leads,
        note: "mydomain.com/monday · on your phone too",
        isLive: true,
    },
    {
        kind: "reply",
        text: "Done. Your Monday page reads your mail, chat, tasks and calendar directly, so it's always current. It's made of the same parts as your other apps, so it works on your phone as they do.",
    },
]);

/** The calls going live fails at: Gmail access that lapses, no database account, and no sync. */
const blocked: readonly Item[] = [
    {
        kind: "call",
        server: "gmail",
        running: "Connecting Gmail and Calendar",
        title: "Gmail access lasts 7 days",
        request:
            'oauth.consent({ scopes: ["gmail.readonly", "calendar.readonly"], status: "Testing" })',
        response:
            "Refresh tokens for apps in Testing expire after 7 days. Publishing with gmail.readonly needs restricted scope verification.",
        isFailed: true,
        isExpanded: true,
    },
    {
        kind: "call",
        server: "supabase",
        running: "Creating a database",
        title: "Couldn't create a database",
        request: 'create_project({ name: "monday", organization_id: "…", region: "eu-central-1" })',
        response: '401 { "message": "Unauthorized" }',
        isFailed: true,
        isExpanded: false,
    },
    {
        kind: "call",
        server: "vercel",
        running: "Scheduling the mail sync",
        title: "Couldn't sync every 5 minutes",
        request: 'crons: [{ path: "/api/sync", schedule: "*/5 * * * *" }]',
        response:
            "Hobby accounts are limited to daily cron jobs. This cron expression (*/5 * * * *) would run more than once per day.",
        isFailed: true,
        isExpanded: false,
    },
];

/** Return the conversation where an agent left the app: made, then three services short of live. */
export function stackedSceneOf(maker: Maker): Scene {
    return Scene.of([
        ask,
        ...madeOf(maker),
        online,
        ...blocked,
        { kind: "reply", text: shortOf(maker) },
    ]);
}

/** Return what your agent shows once it made the app, by where it left it. */
function madeOf(maker: Maker): readonly Item[] {
    // write the files in Amp's sandbox and serve them at its preview address
    const place = maker.place;
    if (place.kind === "sandbox") {
        return [
            {
                kind: "call",
                server: SHELL,
                running: "Editing src/monday.tsx",
                title: "Edited src/monday.tsx +164",
                request: "src/monday.tsx",
                response: "promises from mail · due this week · free slots",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "call",
                server: SHELL,
                running: "Running pnpm dev",
                title: "$ pnpm dev",
                request: "pnpm dev",
                response: `VITE v7.1.3 ready in 412 ms · Local http://localhost:5173/ · Portal https://${place.host}`,
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: `It's running in this orb at ${place.host}, on a sample of your mail for now. Reading your real Gmail and Calendar needs Google sign-in.`,
            },
            {
                kind: "app",
                title: "Monday",
                badge: "Preview",
                rows: leads,
                note: place.host,
                isLive: false,
            },
        ];
    }

    // make it as an artifact, a canvas or a version, its preview inline
    const address = artifactAddressOf(place.assistant);

    return [
        { kind: "reply", text: madeReplyOf() },
        {
            kind: "app",
            title: "Monday",
            badge: "Interactive",
            rows: leads,
            note: `${address.host}${address.path}`,
            isLive: false,
        },
    ];
}

/** Return what your agent says once it made the app as an artifact, canvas or version. */
function madeReplyOf(): string {
    return "Here's your Monday page: what you promised in emails and haven't done, what's due this week, and the free slots in your calendar. It runs on sample data, since an artifact can't read your Gmail.";
}

/** Return what your agent says going live takes, by where it left the app. */
function shortOf(maker: Maker): string {
    // suggest the usual services from where the app is, and offer to set them up
    return maker.place.kind === "artifact"
        ? "To use it on your phone with your real mail, it needs to become a small web app: a Google Cloud project for Gmail and Calendar access, which lapses every 7 days until Google verifies the app, Supabase to keep the copies, and Vercel with a sync every few minutes, which needs Pro at $20 a month. I can write the code and walk you through each. Want to start with Google?"
        : "To run it on your phone with your real mail, it needs Google access for Gmail and Calendar, which lapses every 7 days until Google verifies the app, a database for the copies, and a sync every few minutes. I'd use Supabase and deploy to Vercel Pro at $20 a month for the schedule. You'll create the accounts and add their keys as secrets in this orb. Want me to start with Google?";
}

/** The connectors the stacked agent has on, one per service the app rents, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["github", "#181717"],
    ["gmail", "#ea4335"],
    ["supabase", "#3ecf8e"],
    ["vercel", "#000000"],
    ["clerk", "#6c47ff"],
    ["neon", "#00a87e"],
    ["sentry", "#362d59"],
    ["posthog", "#f54e00"],
];
