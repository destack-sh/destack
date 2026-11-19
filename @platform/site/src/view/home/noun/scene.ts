import { artifactAddressOf } from "../command/artifact";

import { type Connector, SHELL } from "../command/chat";
import { type Item, type Lead, Scene } from "../command/scene";
import type { Maker } from "./maker";
import { milestones } from "../project/tracker";

/** The launch tracker's milestones, as its preview shows them. */
const leads: readonly Lead[] = milestones.map(([title, due, isDone]) => [
    title,
    due,
    isDone ? "Done" : "Open",
]);

/** What you ask your agent to build. */
const ask: Item = {
    kind: "ask",
    text: "we need a launch tracker for the waitlist. signups, milestones, and a reminder email friday 9am",
};

/** What you ask once the app works. */
const launch: Item = { kind: "ask", text: "nice. ok put it online for the team pls" };

/** Your agent building the app in your space, every service it needs included. */
const build: Item = {
    kind: "call",
    server: "destack",
    running: "Building in your space",
    title: "Built launch-tracker",
    request: 'client.app.create({ name: "launch-tracker", from: "./launch-tracker" })',
    response: "sign-in · tables · email · jobs · analytics · errors · hosting · code",
    isFailed: false,
    isExpanded: false,
};

/** The conversation in your space: the app built and live in one go. */
export const destackedScene = Scene.of([
    ask,
    build,
    {
        kind: "app",
        title: "Launch tracker",
        badge: "Live",
        rows: leads,
        note: "mydomain.com/launch-tracker · your team signs in",
        isLive: true,
    },
    {
        kind: "reply",
        text: "Done. I created the launch tracker in your space: signups go into a signups table, your team signs in with their accounts, and the reminder email goes out Fridays at 9:00. It's live at mydomain.com/launch-tracker.",
    },
]);

/** The calls going live fails at: no database account, and no verified domain to send from. */
const blocked: readonly Item[] = [
    {
        kind: "call",
        server: "supabase",
        running: "Creating a database",
        title: "Couldn't create a database",
        request:
            'create_project({ name: "launch-tracker", organization_id: "…", region: "eu-central-1" })',
        response: '401 { "message": "Unauthorized" }',
        isFailed: true,
        isExpanded: true,
    },
    {
        kind: "call",
        server: "resend",
        running: "Sending a test reminder",
        title: "Couldn't send from mydomain.com",
        request:
            'emails.send({ from: "launch@mydomain.com", to: "florian@…", subject: "Launch reminder" })',
        response:
            '403 { "statusCode": 403, "name": "validation_error", "message": "The mydomain.com domain is not verified. Please, add and verify your domain on https://resend.com/domains" }',
        isFailed: true,
        isExpanded: false,
    },
];

/** Return the conversation where an agent left the app: made, then nine services short of live. */
export function stackedSceneOf(maker: Maker): Scene {
    return Scene.of([
        ask,
        ...madeOf(maker),
        launch,
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
                running: "Editing src/tracker.tsx",
                title: "Edited src/tracker.tsx +142",
                request: "src/tracker.tsx",
                response: "signup form · milestones · reminder email draft",
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
                text: `It's running in this orb at ${place.host}. Signups save to a SQLite file inside the orb for now, and the reminder email only logs.`,
            },
            {
                kind: "app",
                title: "Launch tracker",
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
            title: "Launch tracker",
            badge: "Interactive",
            rows: leads,
            note: `${address.host}${address.path}`,
            isLive: false,
        },
    ];
}

/** Return what your agent says once it made the app as an artifact, canvas or version. */
function madeReplyOf(): string {
    return "Here's a launch tracker with a signup form, your milestones, and a draft of the reminder email. Signups only live in this artifact for now, so they reset when it reloads.";
}

/** Return what your agent says going live takes, by where it left the app. */
function shortOf(maker: Maker): string {
    // suggest the usual services from where the app is, and offer to set them up
    return maker.place.kind === "artifact"
        ? "To share it with your team, it needs to become a small web app. A common setup is Supabase for the database and sign-in, Resend for the reminder email, and Vercel for hosting. I can write the code and walk you through creating each account and adding its keys. Want to start with Supabase?"
        : "To put it online for the team, it needs a real database, sign-in and email. I'd use Supabase for the data and auth, Resend for the reminder, and deploy it to Vercel. You'll need to create the three accounts and add their keys as secrets in this orb. Want me to start with Supabase?";
}

/** The connectors the stacked agent has on, one per service the app rents, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["github", "#181717"],
    ["supabase", "#3ecf8e"],
    ["resend", "#000000"],
    ["vercel", "#000000"],
    ["clerk", "#6c47ff"],
    ["neon", "#00a87e"],
    ["sentry", "#362d59"],
    ["posthog", "#f54e00"],
];
