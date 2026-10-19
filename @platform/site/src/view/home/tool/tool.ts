import { present } from "@destack/schema";
import type { JSX } from "@destack/view";

import { Airtable } from "./airtable";
import { Asana } from "./asana";
import { Bolt } from "./bolt";
import { CalCom } from "./calcom";
import { Calendly } from "./calendly";
import { Coda } from "./coda";
import { ClickUp } from "./clickup";
import { Discord } from "./discord";
import { Excalidraw } from "./excalidraw";
import { Ifttt } from "./ifttt";
import { Excel } from "./excel";
import { FigJam } from "./figjam";
import { Confluence } from "./confluence";
import { GoogleChat } from "./googlechat";
import { GoogleCalendar } from "./googlecalendar";
import { GoogleDocuments } from "./googledocs";
import { GoogleSheets } from "./googlesheets";
import { Jira } from "./jira";
import { Obsidian } from "./obsidian";
import { Outlook } from "./outlook";
import { OutlookCalendar } from "./outlookcalendar";
import { PowerAutomate } from "./powerautomate";
import { ProtonCalendar } from "./protoncalendar";
import { ProtonMail } from "./protonmail";
import { Word } from "./word";
import { Gmail } from "./gmail";
import { Linear } from "./linear";
import { Lovable } from "./lovable";
import { Make } from "./make";
import { Lucid } from "./lucid";
import { Miro } from "./miro";
import { N8n } from "./n8n";
import { Notion, NotionDatabase } from "./notion";
import { Replit } from "./replit";
import { Slack } from "./slack";
import { Superhuman } from "./superhuman";
import { Teams } from "./teams";
import { Thunderbird } from "./thunderbird";
import { Tldraw } from "./tldraw";
import { Trello } from "./trello";
import { V0 } from "./v0";
import { WhatsApp } from "./whatsapp";
import { Zapier } from "./zapier";

/** What a team rents a tool for, as it shops for one. */
export type Category =
    | "documents"
    | "tables"
    | "tasks"
    | "calendar"
    | "chat"
    | "mail"
    | "canvas"
    | "workflows"
    | "builder";

/** What your space does with a tool: replaces it with its own app, or keeps it bridged. */
export type Relation = { kind: "replace" } | { kind: "bridge"; status: string };

/** A tool a team rents: its mark, the plan it bills under, and its site drawn in a window. */
export type Tool = {
    /** The tool's name. */
    name: string;
    /** What a team rents it for. */
    category: Category;
    /** The tool's mark in the diagram icons. */
    icon: string;
    /** The tool's brand colour. */
    tint: string;
    /** The subscription it bills under, shared by a suite's tools. */
    subscription: string;
    /** The title of its browser tab. */
    tab: string;
    /** The address of its page. */
    site: string;
    /** What your space does with it. */
    relation: Relation;
    /** Draw its page at the size of a real window. */
    page: () => JSX.Element;
};

/** Every tool the page names, grouped by category, a tool rented for two jobs listed in each. */
export const tools: readonly Tool[] = [
    // docs
    {
        name: "Notion",
        category: "documents",
        icon: "notion",
        tint: "#191919",
        subscription: "Notion",
        tab: "Destack launch",
        site: "florian.notion.site/Destack-launch",
        relation: { kind: "replace" },
        page: Notion,
    },
    {
        name: "Google Docs",
        category: "documents",
        icon: "googledocs",
        tint: "#4285f4",
        subscription: "Google Workspace",
        tab: "Destack launch - Google Docs",
        site: "docs.google.com/document/d/1xDst",
        relation: { kind: "replace" },
        page: GoogleDocuments,
    },
    {
        name: "Confluence",
        category: "documents",
        icon: "confluence",
        tint: "#1868db",
        subscription: "Confluence",
        tab: "Destack launch - Destack - Confluence",
        site: "symbol.atlassian.net/wiki/spaces/DST/pages/launch",
        relation: { kind: "replace" },
        page: Confluence,
    },
    {
        name: "Obsidian",
        category: "documents",
        icon: "obsidian",
        tint: "#7c3aed",
        subscription: "Obsidian",
        tab: "Destack launch - Destack - Obsidian",
        site: "obsidian://open?vault=Destack&file=Launch",
        relation: { kind: "replace" },
        page: Obsidian,
    },
    {
        name: "Word",
        category: "documents",
        icon: "word",
        tint: "#185abd",
        subscription: "Microsoft 365",
        tab: "Destack launch.docx",
        site: "symbol.sharepoint.com/:w:/Destack-launch.docx",
        relation: { kind: "replace" },
        page: Word,
    },

    // tasks
    {
        name: "Linear",
        category: "tasks",
        icon: "linear",
        tint: "#5e6ad2",
        subscription: "Linear",
        tab: "Destack › All issues",
        site: "linear.app/symbol/team/DST/all",
        relation: { kind: "replace" },
        page: Linear,
    },
    {
        name: "Jira",
        category: "tasks",
        icon: "jira",
        tint: "#1868db",
        subscription: "Jira",
        tab: "DST board - Jira",
        site: "symbol.atlassian.net/jira/software/projects/DST/boards/1",
        relation: { kind: "replace" },
        page: Jira,
    },
    {
        name: "Asana",
        category: "tasks",
        icon: "asana",
        tint: "#f06a6a",
        subscription: "Asana",
        tab: "Destack launch - List - Asana",
        site: "app.asana.com/0/1208/list",
        relation: { kind: "replace" },
        page: Asana,
    },
    {
        name: "Trello",
        category: "tasks",
        icon: "trello",
        tint: "#0052cc",
        subscription: "Trello",
        tab: "Destack launch | Trello",
        site: "trello.com/b/dst/destack-launch",
        relation: { kind: "replace" },
        page: Trello,
    },
    {
        name: "ClickUp",
        category: "tasks",
        icon: "clickup",
        tint: "#7b68ee",
        subscription: "ClickUp",
        tab: "Launch | Destack | ClickUp",
        site: "app.clickup.com/9012/v/l/launch",
        relation: { kind: "replace" },
        page: ClickUp,
    },

    // chat
    {
        name: "Slack",
        category: "chat",
        icon: "slack",
        tint: "#4a154b",
        subscription: "Slack",
        tab: "launch (Channel) - Symbol - Slack",
        site: "symbol.slack.com/archives/launch",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Slack,
    },
    {
        name: "Teams",
        category: "chat",
        icon: "teams",
        tint: "#5059c9",
        subscription: "Microsoft 365",
        tab: "launch (Symbol) | Microsoft Teams",
        site: "teams.microsoft.com/v2/symbol/launch",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Teams,
    },
    {
        name: "Discord",
        category: "chat",
        icon: "discord",
        tint: "#5865f2",
        subscription: "Discord",
        tab: "#launch | Symbol - Discord",
        site: "discord.com/channels/symbol/launch",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Discord,
    },
    {
        name: "Google Chat",
        category: "chat",
        icon: "googlechat",
        tint: "#00ac47",
        subscription: "Google Workspace",
        tab: "Launch - Chat",
        site: "chat.google.com/room/launch",
        relation: { kind: "bridge", status: "messages bridged" },
        page: GoogleChat,
    },
    {
        name: "WhatsApp",
        category: "chat",
        icon: "whatsapp",
        tint: "#25d366",
        subscription: "WhatsApp",
        tab: "WhatsApp",
        site: "web.whatsapp.com",
        relation: { kind: "bridge", status: "messages bridged" },
        page: WhatsApp,
    },

    // calendar
    {
        name: "Calendly",
        category: "calendar",
        icon: "calendly",
        tint: "#006bff",
        subscription: "Calendly",
        tab: "Destack demo | Florian | Calendly",
        site: "calendly.com/florian/destack-demo",
        relation: { kind: "replace" },
        page: Calendly,
    },
    {
        name: "Cal.com",
        category: "calendar",
        icon: "caldotcom",
        tint: "#292929",
        subscription: "Cal.com",
        tab: "Destack demo | Florian | Cal.com",
        site: "cal.com/florian/destack-demo",
        relation: { kind: "replace" },
        page: CalCom,
    },
    {
        name: "Google Calendar",
        category: "calendar",
        icon: "googlecalendar",
        tint: "#4285f4",
        subscription: "Google Workspace",
        tab: "Google Calendar - Week of October 20, 2025",
        site: "calendar.google.com/calendar/u/0/r/week/2025/10/20",
        relation: { kind: "bridge", status: "two-way sync" },
        page: GoogleCalendar,
    },
    {
        name: "Outlook Calendar",
        category: "calendar",
        icon: "outlook",
        tint: "#0078d4",
        subscription: "Microsoft 365",
        tab: "Calendar - Florian - Outlook",
        site: "outlook.office.com/calendar/view/workweek",
        relation: { kind: "bridge", status: "two-way sync" },
        page: OutlookCalendar,
    },
    {
        name: "Proton Calendar",
        category: "calendar",
        icon: "protoncalendar",
        tint: "#6d4aff",
        subscription: "Proton Mail",
        tab: "Proton Calendar",
        site: "calendar.proton.me/u/0/week/2025/10/20",
        relation: { kind: "bridge", status: "events read by link" },
        page: ProtonCalendar,
    },

    // mail
    {
        name: "Gmail",
        category: "mail",
        icon: "gmail",
        tint: "#ea4335",
        subscription: "Google Workspace",
        tab: "Inbox (2) - Gmail",
        site: "mail.google.com/mail/u/0",
        relation: { kind: "bridge", status: "inbox synced" },
        page: Gmail,
    },
    {
        name: "Outlook",
        category: "mail",
        icon: "outlook",
        tint: "#0078d4",
        subscription: "Microsoft 365",
        tab: "Mail - Florian - Outlook",
        site: "outlook.office.com/mail/inbox",
        relation: { kind: "bridge", status: "inbox synced" },
        page: Outlook,
    },
    {
        name: "Superhuman",
        category: "mail",
        icon: "superhuman",
        tint: "#3d1832",
        subscription: "Superhuman",
        tab: "Superhuman",
        site: "mail.superhuman.com/florian@symbol.co",
        relation: { kind: "replace" },
        page: Superhuman,
    },
    {
        name: "Thunderbird",
        category: "mail",
        icon: "thunderbird",
        tint: "#0a84ff",
        subscription: "Thunderbird",
        tab: "Inbox - Mozilla Thunderbird",
        site: "mailbox:///florian@symbol.co/Inbox",
        relation: { kind: "replace" },
        page: Thunderbird,
    },
    {
        name: "Proton Mail",
        category: "mail",
        icon: "protonmail",
        tint: "#6d4aff",
        subscription: "Proton Mail",
        tab: "Inbox | Proton Mail",
        site: "mail.proton.me/u/0/inbox",
        relation: { kind: "bridge", status: "inbox synced" },
        page: ProtonMail,
    },

    // tables
    {
        name: "Airtable",
        category: "tables",
        icon: "airtable",
        tint: "#18bfff",
        subscription: "Airtable",
        tab: "Launch: Press - Airtable",
        site: "airtable.com/appLaunch/tblPress",
        relation: { kind: "replace" },
        page: Airtable,
    },
    {
        name: "Google Sheets",
        category: "tables",
        icon: "googlesheets",
        tint: "#34a853",
        subscription: "Google Workspace",
        tab: "Launch – Press - Google Sheets",
        site: "docs.google.com/spreadsheets/d/1xPress",
        relation: { kind: "replace" },
        page: GoogleSheets,
    },
    {
        name: "Excel",
        category: "tables",
        icon: "excel",
        tint: "#107c41",
        subscription: "Microsoft 365",
        tab: "Launch press list.xlsx",
        site: "symbol.sharepoint.com/:x:/Launch-press-list.xlsx",
        relation: { kind: "replace" },
        page: Excel,
    },
    {
        name: "Coda",
        category: "tables",
        icon: "coda",
        tint: "#f46a54",
        subscription: "Coda",
        tab: "Press · Destack launch",
        site: "coda.io/d/Destack-launch/Press",
        relation: { kind: "replace" },
        page: Coda,
    },
    {
        name: "Notion",
        category: "tables",
        icon: "notion",
        tint: "#191919",
        subscription: "Notion",
        tab: "Press",
        site: "notion.so/symbol/Press-8c41f0e2a5b94d1c9e7f3a6b2d5c8e10",
        relation: { kind: "replace" },
        page: NotionDatabase,
    },

    // canvas
    {
        name: "Miro",
        category: "canvas",
        icon: "miro",
        tint: "#050038",
        subscription: "Miro",
        tab: "Destack launch plan, Online Whiteboard for Visual Collaboration",
        site: "miro.com/app/board/uXjVDestack=",
        relation: { kind: "replace" },
        page: Miro,
    },
    {
        name: "FigJam",
        category: "canvas",
        icon: "figma",
        tint: "#a259ff",
        subscription: "FigJam",
        tab: "Destack launch plan – FigJam",
        site: "figma.com/board/Lp7Destack/Destack-launch-plan",
        relation: { kind: "replace" },
        page: FigJam,
    },
    {
        name: "tldraw",
        category: "canvas",
        icon: "tldraw",
        tint: "#1d1d1d",
        subscription: "tldraw",
        tab: "Destack launch plan | tldraw",
        site: "www.tldraw.com/f/qY7rBw2hT1kZ",
        relation: { kind: "replace" },
        page: Tldraw,
    },
    {
        name: "Lucid",
        category: "canvas",
        icon: "lucid",
        tint: "#282c33",
        subscription: "Lucid",
        tab: "Destack launch plan: Lucidspark",
        site: "lucid.app/lucidspark/destack-launch-plan/edit",
        relation: { kind: "replace" },
        page: Lucid,
    },
    {
        name: "Excalidraw",
        category: "canvas",
        icon: "excalidraw",
        tint: "#6965db",
        subscription: "Excalidraw",
        tab: "Destack launch plan | Excalidraw+",
        site: "app.excalidraw.com/s/3Destack/launch-plan",
        relation: { kind: "replace" },
        page: Excalidraw,
    },

    // workflows
    {
        name: "Zapier",
        category: "workflows",
        icon: "zapier",
        tint: "#ff4f00",
        subscription: "Zapier",
        tab: "Welcome new signups | Zapier",
        site: "zapier.com/editor/248117/published",
        relation: { kind: "replace" },
        page: Zapier,
    },
    {
        name: "Make",
        category: "workflows",
        icon: "make",
        tint: "#6d00cc",
        subscription: "Make",
        tab: "Welcome new signups | Make",
        site: "eu1.make.com/248117/scenarios/4821/edit",
        relation: { kind: "replace" },
        page: Make,
    },
    {
        name: "n8n",
        category: "workflows",
        icon: "n8n",
        tint: "#ea4b71",
        subscription: "n8n",
        tab: "Welcome new signups - n8n",
        site: "symbol.app.n8n.cloud/workflow/Kd82Welcome",
        relation: { kind: "replace" },
        page: N8n,
    },
    {
        name: "Power Automate",
        category: "workflows",
        icon: "powerautomate",
        tint: "#0066ff",
        subscription: "Microsoft 365",
        tab: "Welcome new signups | Power Automate",
        site: "make.powerautomate.com/environments/Default/flows/welcome",
        relation: { kind: "replace" },
        page: PowerAutomate,
    },
    {
        name: "IFTTT",
        category: "workflows",
        icon: "ifttt",
        tint: "#000000",
        subscription: "IFTTT",
        tab: "Post new waitlist signups to #launch and welcome them by email - IFTTT",
        site: "ifttt.com/applets/Rk3vT9qa-post-new-waitlist-signups-to-launch",
        relation: { kind: "replace" },
        page: Ifttt,
    },

    // builder
    {
        name: "Replit",
        category: "builder",
        icon: "replit",
        tint: "#f26207",
        subscription: "Replit",
        tab: "Join the waitlist",
        site: "destack-waitlist.replit.app",
        relation: { kind: "replace" },
        page: Replit,
    },
    {
        name: "Lovable",
        category: "builder",
        icon: "lovable",
        tint: "#ff4f6d",
        subscription: "Lovable",
        tab: "Destack · Join the waitlist",
        site: "destack-waitlist.lovable.app",
        relation: { kind: "replace" },
        page: Lovable,
    },
    {
        name: "Bolt",
        category: "builder",
        icon: "stackblitz",
        tint: "#1389fd",
        subscription: "Bolt",
        tab: "Destack waitlist",
        site: "destack-waitlist.bolt.host",
        relation: { kind: "replace" },
        page: Bolt,
    },
    {
        name: "v0",
        category: "builder",
        icon: "v0",
        tint: "#000000",
        subscription: "v0",
        tab: "Destack – Join the waitlist",
        site: "destack-waitlist.vercel.app",
        relation: { kind: "replace" },
        page: V0,
    },
];

/** The tool each category shows. */
export type Lineup = (category: Category) => Tool;

/** Return the lineup a number of rounds in, each category turned to its next tool every round. */
export function lineupAt(round: number): Lineup {
    return (category) => {
        const turning = toolsOf(category);

        return present(turning[round % turning.length], `tool for ${category}`);
    };
}

/** Return the tools of a category, in the catalog's order. */
export function toolsOf(category: Category): readonly Tool[] {
    return tools.filter((tool) => tool.category === category);
}

/** Return a tool's first entry by its name, the job it is rented for first. */
export function toolOf(name: string): Tool {
    return present(
        tools.find((tool) => tool.name === name),
        `tool ${name}`,
    );
}
