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
} as const;

/** The currencies Destack bills in. */
export const currencies = ["USD", "CAD", "EUR", "CHF", "GBP", "AUD"] as const;

/** A US dollar of rate in each currency. */
export const creditPrices = { USD: 1, CAD: 1.4, EUR: 1, CHF: 1, GBP: 1, AUD: 1.5 } as const;

/** The plans, cheapest first, each fee a month in each currency. */
export const plans = [
    {
        key: "free",
        name: "Free",
        fees: { USD: 0, CAD: 0, EUR: 0, CHF: 0, GBP: 0, AUD: 0 },
        support: "community",
        retention: 7,
        isUncapped: false,
        isCustomDomains: false,
        isEarlyAccess: false,
        isDedicated: false,
    },
    {
        key: "go",
        name: "Go",
        fees: { USD: 8, CAD: 11, EUR: 8, CHF: 8, GBP: 8, AUD: 12 },
        support: "community",
        retention: 30,
        isUncapped: true,
        isCustomDomains: false,
        isEarlyAccess: true,
        isDedicated: false,
    },
    {
        key: "plus",
        name: "Plus",
        fees: { USD: 20, CAD: 28, EUR: 20, CHF: 20, GBP: 20, AUD: 30 },
        support: "email",
        retention: 90,
        isUncapped: true,
        isCustomDomains: true,
        isEarlyAccess: true,
        isDedicated: false,
    },
    {
        key: "pro",
        name: "Pro",
        fees: { USD: 100, CAD: 140, EUR: 100, CHF: 100, GBP: 100, AUD: 150 },
        support: "priority",
        retention: 365,
        isUncapped: true,
        isCustomDomains: true,
        isEarlyAccess: true,
        isDedicated: true,
    },
] as const;

/** The free plan's monthly allowance, in GB and GB-hours. */
export const allowance = { database: 1, files: 10, compute: 12.5, egress: 100 } as const;

/** The dedicated machines, smallest first, priced in US dollars. */
export const machines = [
    { size: "small", vcpus: 2, memory: 4, hourly: 0.08, monthly: 40 },
    { size: "medium", vcpus: 4, memory: 8, hourly: 0.16, monthly: 80 },
    { size: "large", vcpus: 8, memory: 16, hourly: 0.32, monthly: 160 },
    { size: "xlarge", vcpus: 16, memory: 32, hourly: 0.64, monthly: 320 },
] as const;
