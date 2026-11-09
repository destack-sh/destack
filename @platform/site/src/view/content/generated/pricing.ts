/** The usage rates in US dollars, per GB-month, GB-hour or million. */
export const rates = {
    database: 0.4,
    files: 0.03,
    memory: 0.09,
    cpu: 0.04,
    requests: 0.9,
    rowsRead: 0.002,
    rowsWritten: 2,
    fileReads: 0.72,
    fileWrites: 9,
    inference: 1.25,
} as const;

/** The currencies Destack bills in. */
export const currencies = ["USD", "CAD", "EUR", "CHF", "GBP", "AUD"] as const;

/** A US dollar of rate in each currency. */
export const creditPrices = { USD: 1, CAD: 1.4, EUR: 1, CHF: 1, GBP: 1, AUD: 1.5 } as const;

/** The plans, cheapest first, then the enterprise contract, each fee a month in each currency. */
export const plans = [
    {
        key: "free",
        name: "Free",
        fees: { USD: 0, CAD: 0, EUR: 0, CHF: 0, GBP: 0, AUD: 0 },
        support: "community",
        isUncapped: false,
        isCustomDomains: false,
        isCustomRetention: false,
        isEarlyAccess: false,
        isDedicated: false,
        isContract: false,
    },
    {
        key: "go",
        name: "Go",
        fees: { USD: 8, CAD: 11, EUR: 8, CHF: 8, GBP: 8, AUD: 13 },
        support: "community",
        isUncapped: false,
        isCustomDomains: false,
        isCustomRetention: true,
        isEarlyAccess: true,
        isDedicated: false,
        isContract: false,
    },
    {
        key: "plus",
        name: "Plus",
        fees: { USD: 20, CAD: 28, EUR: 20, CHF: 20, GBP: 20, AUD: 32 },
        support: "email",
        isUncapped: true,
        isCustomDomains: true,
        isCustomRetention: true,
        isEarlyAccess: true,
        isDedicated: false,
        isContract: false,
    },
    {
        key: "pro",
        name: "Pro",
        fees: { USD: 50, CAD: 70, EUR: 50, CHF: 50, GBP: 50, AUD: 80 },
        support: "priority",
        isUncapped: true,
        isCustomDomains: true,
        isCustomRetention: true,
        isEarlyAccess: true,
        isDedicated: true,
        isContract: false,
    },
    {
        key: "enterprise",
        name: "Enterprise",
        fees: { USD: 1000, CAD: 1400, EUR: 1000, CHF: 1000, GBP: 1000, AUD: 1600 },
        support: "sla",
        isUncapped: true,
        isCustomDomains: true,
        isCustomRetention: true,
        isEarlyAccess: true,
        isDedicated: true,
        isContract: true,
    },
] as const;

/** The free plan's usage per member a month in US dollars, and a month it covers. */
export const allowance = { usage: 2.5, database: 1, files: 10, compute: 10, requests: 1 } as const;

/** The dedicated machines, smallest first, priced in US dollars. */
export const machines = [
    { size: "small", vcpus: 2, memory: 4, disk: 80, hourly: 0.08, monthly: 40 },
    { size: "medium", vcpus: 4, memory: 8, disk: 160, hourly: 0.16, monthly: 80 },
    { size: "large", vcpus: 8, memory: 16, disk: 320, hourly: 0.32, monthly: 160 },
    { size: "xlarge", vcpus: 16, memory: 32, disk: 640, hourly: 0.64, monthly: 320 },
] as const;
