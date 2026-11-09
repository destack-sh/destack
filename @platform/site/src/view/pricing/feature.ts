/** A feature the plans list: its name, and what it means. */
export type Feature = { name: string; detail: string };

/** Every feature the plan cards and the plan comparison list, under one name each. */
export const features = {
    // software
    members: {
        name: "Unlimited members",
        detail: "Invite as many people and guests as you like, with no per-seat fee.",
    },
    agents: {
        name: "Unlimited agents",
        detail: "Run as many agents as you like, with no per-agent fee; their models bill as usage.",
    },
    apps: {
        name: "Built-in apps",
        detail: "Every app that ships with Destack, with no seats or licences to pay.",
    },
    integrations: {
        name: "Integrations",
        detail: "Connect the services you already use, with no fee per connection.",
    },
    branches: {
        name: "Branches and previews",
        detail: "Every app branches, with a preview for each branch.",
    },
    access: {
        name: "Access control",
        detail: "One set of roles for the people, guests and agents in your spaces.",
    },
    secrets: { name: "Secrets", detail: "Keys and passwords, encrypted and shared." },
    audit: { name: "Audit log", detail: "Every change, by whom and when." },
    observability: { name: "Observability", detail: "Logs, traces and alerts." },

    // hosting
    selfHosting: {
        name: "Self-hosting",
        detail: "Run your spaces on your own machines, with no fee for the software.",
    },
    cloudHosting: {
        name: "Cloud hosting",
        detail: "Run your spaces on our cloud, on machines shared with other accounts.",
    },
    tunnels: { name: "Tunnels", detail: "Reach your machines from anywhere." },
    domains: {
        name: "Custom domains",
        detail: "Serve your spaces and your profile under your own domain names.",
    },
    dedicated: {
        name: "Dedicated machines",
        detail: "A machine of your own, billed by the hour up to a monthly cap.",
    },

    // ai
    ownAi: {
        name: "Bring your own AI",
        detail: "Use your subscriptions, your API keys or local models.",
    },
    managedAi: {
        name: "Managed AI",
        detail: "Use models through us, with no keys or accounts to manage, billed as usage.",
    },

    // usage
    included: { name: "Included usage", detail: "Usage is billed at the unit prices below." },
    uncapped: {
        name: "No usage caps",
        detail: "Your apps never slow down: past the included usage, you pay as you go.",
    },
    retention: {
        name: "Retention",
        detail: "How long logs, traces and activity stay: their default spans, or as long as you like.",
    },
    defaultRetention: {
        name: "Default retention",
        detail: "Logs, traces and activity stay for their default spans, or shorter if you like.",
    },
    customRetention: {
        name: "Custom retention",
        detail: "Keep logs, traces and activity as long as you like. They count as storage.",
    },

    // support
    support: {
        name: "Support",
        detail: "Community on Discord and GitHub, email, priority, or guaranteed response times.",
    },
    communitySupport: { name: "Community support", detail: "Ask on Discord and GitHub." },
    emailSupport: { name: "Email support", detail: "Write to the people who build Destack." },
    prioritySupport: {
        name: "Priority support",
        detail: "Your questions go to the front of the queue.",
    },
    slaSupport: {
        name: "SLA support",
        detail: "Guaranteed response times from engineers who know your stack.",
    },
    early: { name: "Early access", detail: "Try new apps and features before everyone else." },
    engineer: {
        name: "Dedicated engineer",
        detail: "An engineer of ours moves your data and tools over and helps build your apps.",
    },

    // contract
    uptime: {
        name: "Uptime SLA",
        detail: "An uptime we promise in your contract, with credits when we miss it.",
    },
    invoice: {
        name: "Invoice billing",
        detail: "Pay by invoice and bank transfer, with purchase orders.",
    },
    agreement: {
        name: "Data processing agreement",
        detail: "A signed agreement on how we process your data, for your privacy reviews.",
    },
} as const satisfies Readonly<Record<string, Feature>>;
