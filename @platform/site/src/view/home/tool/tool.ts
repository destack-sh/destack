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
    /** The tool's name in a row of its category, its brand left to its mark, such as Docs. */
    label: string;
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
        label: "Notion",
        category: "documents",
        icon: "notion",
        tint: "#191919",
        subscription: "Notion",
        tab: "Ines Ferreira",
        site: "florian.notion.site/Ines-Ferreira",
        relation: { kind: "replace" },
        page: Notion,
    },
    {
        name: "Google Docs",
        label: "Docs",
        category: "documents",
        icon: "googledocs",
        tint: "#4285f4",
        subscription: "Google Workspace",
        tab: "Ines Ferreira - Google Docs",
        site: "docs.google.com/document/d/1xDst",
        relation: { kind: "replace" },
        page: GoogleDocuments,
    },
    {
        name: "Confluence",
        label: "Confluence",
        category: "documents",
        icon: "confluence",
        tint: "#1868db",
        subscription: "Confluence",
        tab: "Ines Ferreira - Personal - Confluence",
        site: "florian.atlassian.net/wiki/spaces/ME/pages/ines-ferreira",
        relation: { kind: "replace" },
        page: Confluence,
    },
    {
        name: "Obsidian",
        label: "Obsidian",
        category: "documents",
        icon: "obsidian",
        tint: "#7c3aed",
        subscription: "Obsidian",
        tab: "Ines Ferreira - Personal - Obsidian",
        site: "obsidian://open?vault=Personal&file=Clients/Ines Ferreira",
        relation: { kind: "replace" },
        page: Obsidian,
    },
    {
        name: "Word",
        label: "Word",
        category: "documents",
        icon: "word",
        tint: "#185abd",
        subscription: "Microsoft 365",
        tab: "Ines Ferreira.docx",
        site: "florian-my.sharepoint.com/:w:/Ines-Ferreira.docx",
        relation: { kind: "replace" },
        page: Word,
    },

    // tasks
    {
        name: "Linear",
        label: "Linear",
        category: "tasks",
        icon: "linear",
        tint: "#5e6ad2",
        subscription: "Linear",
        tab: "Work › All issues",
        site: "linear.app/florian/team/T/all",
        relation: { kind: "replace" },
        page: Linear,
    },
    {
        name: "Jira",
        label: "Jira",
        category: "tasks",
        icon: "jira",
        tint: "#1868db",
        subscription: "Jira",
        tab: "T board - Jira",
        site: "florian.atlassian.net/jira/software/projects/T/boards/1",
        relation: { kind: "replace" },
        page: Jira,
    },
    {
        name: "Asana",
        label: "Asana",
        category: "tasks",
        icon: "asana",
        tint: "#f06a6a",
        subscription: "Asana",
        tab: "Work - List - Asana",
        site: "app.asana.com/0/1208/list",
        relation: { kind: "replace" },
        page: Asana,
    },
    {
        name: "Trello",
        label: "Trello",
        category: "tasks",
        icon: "trello",
        tint: "#0052cc",
        subscription: "Trello",
        tab: "Personal | Trello",
        site: "trello.com/b/t/personal",
        relation: { kind: "replace" },
        page: Trello,
    },
    {
        name: "ClickUp",
        label: "ClickUp",
        category: "tasks",
        icon: "clickup",
        tint: "#7b68ee",
        subscription: "ClickUp",
        tab: "Work | Personal | ClickUp",
        site: "app.clickup.com/9012/v/l/work",
        relation: { kind: "replace" },
        page: ClickUp,
    },

    // chat
    {
        name: "Slack",
        label: "Slack",
        category: "chat",
        icon: "slack",
        tint: "#4a154b",
        subscription: "Slack",
        tab: "booking-app (Channel) - Florian - Slack",
        site: "florian.slack.com/archives/booking-app",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Slack,
    },
    {
        name: "Teams",
        label: "Teams",
        category: "chat",
        icon: "teams",
        tint: "#5059c9",
        subscription: "Microsoft 365",
        tab: "booking-app (Florian) | Microsoft Teams",
        site: "teams.microsoft.com/v2/florian/booking-app",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Teams,
    },
    {
        name: "Discord",
        label: "Discord",
        category: "chat",
        icon: "discord",
        tint: "#5865f2",
        subscription: "Discord",
        tab: "#booking-app | Florian - Discord",
        site: "discord.com/channels/florian/booking-app",
        relation: { kind: "bridge", status: "messages bridged" },
        page: Discord,
    },
    {
        name: "Google Chat",
        label: "Chat",
        category: "chat",
        icon: "googlechat",
        tint: "#00ac47",
        subscription: "Google Workspace",
        tab: "Booking app - Chat",
        site: "chat.google.com/room/booking-app",
        relation: { kind: "bridge", status: "messages bridged" },
        page: GoogleChat,
    },
    {
        name: "WhatsApp",
        label: "WhatsApp",
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
        label: "Calendly",
        category: "calendar",
        icon: "calendly",
        tint: "#006bff",
        subscription: "Calendly",
        tab: "Call with Florian | Calendly",
        site: "calendly.com/florian/call",
        relation: { kind: "replace" },
        page: Calendly,
    },
    {
        name: "Cal.com",
        label: "Cal.com",
        category: "calendar",
        icon: "caldotcom",
        tint: "#292929",
        subscription: "Cal.com",
        tab: "Call with Florian | Cal.com",
        site: "cal.com/florian/call",
        relation: { kind: "replace" },
        page: CalCom,
    },
    {
        name: "Google Calendar",
        label: "Calendar",
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
        label: "Outlook",
        category: "calendar",
        icon: "outlook",
        tint: "#0078d4",
        subscription: "Microsoft 365",
        tab: "Calendar - Florian - Outlook",
        site: "outlook.office.com/calendar/view/workweek",
        relation: { kind: "bridge", status: "two-way sync" },
        page: OutlookCalendar,
    },

    // mail
    {
        name: "Gmail",
        label: "Gmail",
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
        label: "Outlook",
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
        label: "Superhuman",
        category: "mail",
        icon: "superhuman",
        tint: "#3d1832",
        subscription: "Superhuman",
        tab: "Superhuman",
        site: "mail.superhuman.com/inbox",
        relation: { kind: "replace" },
        page: Superhuman,
    },
    {
        name: "Thunderbird",
        label: "Thunderbird",
        category: "mail",
        icon: "thunderbird",
        tint: "#0a84ff",
        subscription: "Thunderbird",
        tab: "Inbox - Mozilla Thunderbird",
        site: "mailbox:///Inbox",
        relation: { kind: "replace" },
        page: Thunderbird,
    },

    // tables
    {
        name: "Airtable",
        label: "Airtable",
        category: "tables",
        icon: "airtable",
        tint: "#18bfff",
        subscription: "Airtable",
        tab: "Personal: People - Airtable",
        site: "airtable.com/appPersonal/tblPeople",
        relation: { kind: "replace" },
        page: Airtable,
    },
    {
        name: "Google Sheets",
        label: "Sheets",
        category: "tables",
        icon: "googlesheets",
        tint: "#34a853",
        subscription: "Google Workspace",
        tab: "People - Google Sheets",
        site: "docs.google.com/spreadsheets/d/1xPeople",
        relation: { kind: "replace" },
        page: GoogleSheets,
    },
    {
        name: "Excel",
        label: "Excel",
        category: "tables",
        icon: "excel",
        tint: "#107c41",
        subscription: "Microsoft 365",
        tab: "People.xlsx",
        site: "florian-my.sharepoint.com/:x:/People.xlsx",
        relation: { kind: "replace" },
        page: Excel,
    },
    {
        name: "Coda",
        label: "Coda",
        category: "tables",
        icon: "coda",
        tint: "#f46a54",
        subscription: "Coda",
        tab: "People · Personal",
        site: "coda.io/d/Personal/People",
        relation: { kind: "replace" },
        page: Coda,
    },
    {
        name: "Notion",
        label: "Notion",
        category: "tables",
        icon: "notion",
        tint: "#191919",
        subscription: "Notion",
        tab: "People",
        site: "notion.so/florian/People-8c41f0e2a5b94d1c9e7f3a6b2d5c8e10",
        relation: { kind: "replace" },
        page: NotionDatabase,
    },

    // canvas
    {
        name: "FigJam",
        label: "FigJam",
        category: "canvas",
        icon: "figma",
        tint: "#a259ff",
        subscription: "FigJam",
        tab: "This week – FigJam",
        site: "figma.com/board/Lp7Week/This-week",
        relation: { kind: "replace" },
        page: FigJam,
    },
    {
        name: "Miro",
        label: "Miro",
        category: "canvas",
        icon: "miro",
        tint: "#050038",
        subscription: "Miro",
        tab: "This week, Online Whiteboard for Visual Collaboration",
        site: "miro.com/app/board/uXjVWeek=",
        relation: { kind: "replace" },
        page: Miro,
    },
    {
        name: "tldraw",
        label: "tldraw",
        category: "canvas",
        icon: "tldraw",
        tint: "#1d1d1d",
        subscription: "tldraw",
        tab: "This week | tldraw",
        site: "www.tldraw.com/f/qY7rBw2hT1kZ",
        relation: { kind: "replace" },
        page: Tldraw,
    },
    {
        name: "Lucid",
        label: "Lucid",
        category: "canvas",
        icon: "lucid",
        tint: "#282c33",
        subscription: "Lucid",
        tab: "This week: Lucidspark",
        site: "lucid.app/lucidspark/this-week/edit",
        relation: { kind: "replace" },
        page: Lucid,
    },
    {
        name: "Excalidraw",
        label: "Excalidraw",
        category: "canvas",
        icon: "excalidraw",
        tint: "#6965db",
        subscription: "Excalidraw",
        tab: "This week | Excalidraw+",
        site: "app.excalidraw.com/s/3Week/this-week",
        relation: { kind: "replace" },
        page: Excalidraw,
    },

    // workflows
    {
        name: "Zapier",
        label: "Zapier",
        category: "workflows",
        icon: "zapier",
        tint: "#ff4f00",
        subscription: "Zapier",
        tab: "Confirm new bookings | Zapier",
        site: "zapier.com/editor/248117/published",
        relation: { kind: "replace" },
        page: Zapier,
    },
    {
        name: "Make",
        label: "Make",
        category: "workflows",
        icon: "make",
        tint: "#6d00cc",
        subscription: "Make",
        tab: "Confirm new bookings | Make",
        site: "eu1.make.com/248117/scenarios/4821/edit",
        relation: { kind: "replace" },
        page: Make,
    },
    {
        name: "n8n",
        label: "n8n",
        category: "workflows",
        icon: "n8n",
        tint: "#ea4b71",
        subscription: "n8n",
        tab: "Confirm new bookings - n8n",
        site: "florian.app.n8n.cloud/workflow/Kd82Booking",
        relation: { kind: "replace" },
        page: N8n,
    },
    {
        name: "Power Automate",
        label: "Power Automate",
        category: "workflows",
        icon: "powerautomate",
        tint: "#0066ff",
        subscription: "Microsoft 365",
        tab: "Confirm new bookings | Power Automate",
        site: "make.powerautomate.com/environments/Default/flows/booking",
        relation: { kind: "replace" },
        page: PowerAutomate,
    },
    {
        name: "IFTTT",
        label: "IFTTT",
        category: "workflows",
        icon: "ifttt",
        tint: "#000000",
        subscription: "IFTTT",
        tab: "Add new bookings to People and confirm them by email - IFTTT",
        site: "ifttt.com/applets/Rk3vT9qa-add-new-bookings-to-people",
        relation: { kind: "replace" },
        page: Ifttt,
    },

    // builder
    {
        name: "Replit",
        label: "Replit",
        category: "builder",
        icon: "replit",
        tint: "#f26207",
        subscription: "Replit",
        tab: "Monday",
        site: "monday-florian.replit.app",
        relation: { kind: "replace" },
        page: Replit,
    },
    {
        name: "Lovable",
        label: "Lovable",
        category: "builder",
        icon: "lovable",
        tint: "#ff4f6d",
        subscription: "Lovable",
        tab: "Monday · Your week",
        site: "monday-florian.lovable.app",
        relation: { kind: "replace" },
        page: Lovable,
    },
    {
        name: "Bolt",
        label: "Bolt",
        category: "builder",
        icon: "stackblitz",
        tint: "#1389fd",
        subscription: "Bolt",
        tab: "Monday",
        site: "monday-florian.bolt.host",
        relation: { kind: "replace" },
        page: Bolt,
    },
    {
        name: "v0",
        label: "v0",
        category: "builder",
        icon: "v0",
        tint: "#000000",
        subscription: "v0",
        tab: "Monday – Your week",
        site: "monday-florian.vercel.app",
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
