import type { GlyphName } from "../home/figure/glyph";

/** A feature the plans list: its name, what it means, and the icon it is listed with. */
export type Feature = { name: string; detail: string; icon: GlyphName };

/** Every feature the plan cards and the plan comparison list, under one name each. */
export const features = {
    // software
    members: {
        name: "Members and guests",
        detail: "Any number of members, each on the plan's price, and guests in your spaces for free.",
        icon: "people",
    },
    pooled: {
        name: "Pooled usage",
        detail: "Every member's fee is usage, and an organisation's members share one pool.",
        icon: "layers",
    },
    agents: {
        name: "Unlimited agents",
        detail: "Run as many agents as you like, with no per-agent fee; their models bill as usage.",
        icon: "bot",
    },
    apps: {
        name: "Built-in apps",
        detail: "Every app that ships with Destack, with no seats or licences to pay.",
        icon: "grid",
    },
    integrations: {
        name: "Integrations",
        detail: "Connect the services you already use, with no fee per connection.",
        icon: "puzzle",
    },
    branches: {
        name: "Branches and previews",
        detail: "Every app branches, with a preview for each branch.",
        icon: "remix",
    },
    access: {
        name: "Roles and permissions",
        detail: "One set of roles for the people, guests and agents in your spaces.",
        icon: "key",
    },
    secrets: { name: "Secrets", detail: "Keys and passwords, encrypted and shared.", icon: "lock" },
    audit: { name: "Audit log", detail: "Every change, by whom and when.", icon: "file" },
    observability: {
        name: "Logs and traces",
        detail: "Logs, traces and alerts.",
        icon: "waveform",
    },

    // hosting
    selfHosting: {
        name: "Self-hosting",
        detail: "Run your spaces on your own machines, with no fee for the software.",
        icon: "monitor",
    },
    cloudHosting: {
        name: "Cloud hosting",
        detail: "Run your spaces on our cloud, on machines shared with other accounts.",
        icon: "server",
    },
    residency: {
        name: "Data residency",
        detail: "Choose whether your data stays in the European Union or the United States.",
        icon: "globe",
    },
    tunnels: {
        name: "Tunnels to your machines",
        detail: "Reach your machines from anywhere.",
        icon: "open",
    },
    domains: {
        name: "Custom domains",
        detail: "Serve your spaces and your profile under your own domain names.",
        icon: "link",
    },
    dedicated: {
        name: "Dedicated machines",
        detail: "A machine of your own, billed by the hour up to a monthly cap.",
        icon: "chip",
    },

    // ai
    ownAi: {
        name: "Bring your own AI",
        detail: "Use your subscriptions, your API keys or local models.",
        icon: "plug",
    },
    managedAi: {
        name: "Managed AI",
        detail: "Use models through us, with no keys or accounts to manage, billed as usage.",
        icon: "cloud",
    },

    // usage
    included: {
        name: "Included usage",
        detail: "Usage is billed at the unit prices below.",
        icon: "inbox",
    },
    uncapped: {
        name: "Pay as you go",
        detail: "Keep going past the included usage at the unit prices, up to a monthly limit you set, instead of stopping writes.",
        icon: "up",
    },
    retention: {
        name: "Retention",
        detail: "How long logs, traces and activity stay: their default spans, or as long as you like.",
        icon: "clock",
    },
    defaultRetention: {
        name: "Default retention",
        detail: "Logs, traces and activity stay for their default spans, or shorter if you like.",
        icon: "clock",
    },
    customRetention: {
        name: "Custom retention",
        detail: "Keep logs, traces and activity as long as you like. They count as storage.",
        icon: "clock",
    },

    // support
    support: {
        name: "Support",
        detail: "Community on Discord and GitHub, email, priority, or guaranteed response times.",
        icon: "help",
    },
    communitySupport: {
        name: "Community support",
        detail: "Ask on Discord and GitHub.",
        icon: "people",
    },
    emailSupport: {
        name: "Email support",
        detail: "Write to the people who build Destack.",
        icon: "mail",
    },
    prioritySupport: {
        name: "Priority support",
        detail: "Your questions go to the front of the queue.",
        icon: "bolt",
    },
    slaSupport: {
        name: "SLA support",
        detail: "Guaranteed response times from engineers who know your stack.",
        icon: "help",
    },
    early: {
        name: "Early access",
        detail: "Try new apps and features before everyone else.",
        icon: "star",
    },

    // enterprise
    uptime: {
        name: "Uptime SLA",
        detail: "An uptime we promise in your contract, with credits when we miss it.",
        icon: "check",
    },
    engineer: {
        name: "Dedicated engineer",
        detail: "An engineer of ours moves your data and tools over and helps build your apps.",
        icon: "person",
    },
    invoice: {
        name: "Invoice billing",
        detail: "Pay by invoice and bank transfer, with purchase orders.",
        icon: "file",
    },
    agreement: {
        name: "Data processing agreement",
        detail: "A signed agreement on how we process your data, for your privacy reviews.",
        icon: "lock",
    },
} as const satisfies Readonly<Record<string, Feature>>;
