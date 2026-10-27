import { artifactAddressOf } from "../command/artifact";
import { present } from "@destack/schema";

import { SHELL } from "../command/chat";
import { type Item, type Lead, Scene } from "../command/scene";
import type { Maker } from "./maker";

/** The launch tracker's milestones, as its preview shows them. */
const milestones: readonly Lead[] = [
    ["Pricing page", "Mon 20", "Done"],
    ["Record the demo", "Wed 22", "Open"],
    ["Email the waitlist", "Thu 23", "Open"],
    ["Go live", "Fri 24", "Open"],
];

/** What you ask your agent to build. */
const ask: Item = {
    kind: "ask",
    text: "build a launch tracker for our waitlist, with signups and a reminder email",
};

/** What you ask once the app works. */
const launch: Item = { kind: "ask", text: "nice, now put it online for the team" };

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
        rows: milestones,
        note: "mydomain.com/launch-tracker · your team signs in",
        isLive: true,
    },
    {
        kind: "reply",
        text: "Done. I created the launch tracker in your space: signups go into a signups table, your team signs in with their accounts, and the reminder email goes out Fridays at 9:00. It's live at mydomain.com/launch-tracker.",
    },
]);

/** Return the conversation where an agent left the app: made, then nine services short of live. */
export function stackedSceneOf(maker: Maker): Scene {
    return Scene.of([ask, ...madeOf(maker), launch, { kind: "reply", text: shortOf(maker) }]);
}

/** Return what your agent shows once it made the app, by where it left it. */
function madeOf(maker: Maker): readonly Item[] {
    // write the files and start the dev server on your computer
    const place = maker.place;
    if (place.kind === "localhost") {
        return [
            {
                kind: "call",
                server: SHELL,
                running: "Writing src/tracker.tsx",
                title: "Write(src/tracker.tsx)",
                request: "src/tracker.tsx",
                response: "Wrote 142 lines",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "call",
                server: SHELL,
                running: "Running npm run dev",
                title: "Bash(npm run dev)",
                request: "npm run dev",
                response: "Local: http://localhost:5173/",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: "Done. It's running at localhost:5173. Signups save to a local SQLite file, and the reminder email prints to the console for now.",
            },
        ];
    }

    // make it as an artifact, a canvas or a version, its preview inline
    const address =
        place.kind === "artifact"
            ? artifactAddressOf(place.assistant)
            : { host: place.host, path: "" };

    return [
        { kind: "reply", text: madeReplyOf(maker) },
        {
            kind: "app",
            title: "Launch tracker",
            badge: place.kind === "artifact" ? "Interactive" : "Preview",
            rows: milestones,
            note: `${address.host}${address.path}`,
            isLive: false,
        },
    ];
}

/** Return what your agent says once it made the app as an artifact, canvas or version. */
function madeReplyOf(maker: Maker): string {
    // tell where the signups live for now
    return maker.place.kind === "artifact"
        ? "Here's a launch tracker with a signup form, your milestones, and a draft of the reminder email. Signups only live in this artifact for now, so they reset when it reloads."
        : "Here's the launch tracker, with a signup form, your milestones, and the reminder email.";
}

/** What each platform's agent says going live takes, by the platform's name. */
const platformReplies: Readonly<Record<string, string>> = {
    Lovable:
        "Connect Supabase from the Supabase button and I'll set up the signups table and sign-in. For the reminder email I'll add an edge function that sends through Resend, which needs your Resend API key. Then Publish puts it at launch-tracker.lovable.app.",
    v0: "I can deploy it to Vercel. For signups and sign-in, add Supabase from the Vercel Marketplace, and Resend for the email. Once their keys are in your project, I'll wire them in.",
    Bolt: "Connect Supabase and I'll set up the signups table and sign-in. For the reminder email I'll use Resend, which needs an API key. Then you can deploy it from Bolt.",
    Replit: "Replit's database and Auth can hold the signups and sign-in. For the reminder email I'll add Resend, which needs an API key in Secrets. Then Deploy puts it at launch-tracker.replit.app.",
};

/** Return what your agent says going live takes, by where it left the app. */
function shortOf(maker: Maker): string {
    // suggest the usual services from where the app is, and offer to set them up
    if (maker.place.kind === "artifact") {
        return "To share it with your team, it needs to become a small web app. A common setup is Supabase for the database and sign-in, Resend for the reminder email, and Vercel for hosting. I can write the code and walk you through creating each account and adding its keys. Want to start with Supabase?";
    } else if (maker.place.kind === "localhost") {
        return "To deploy it, I'd move the data to Supabase (Postgres and auth), send the email through Resend, and host it on Vercel. I can add the clients and a deploy config; you'll need to create the three accounts and put their keys in .env. Should I start?";
    }

    return present(platformReplies[maker.name], `reply for ${maker.name}`);
}
