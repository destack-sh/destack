/** The homepage's usage rates in US dollars, per GB-month, GB-hour or million. */
export const rates = {
    database: 0.25,
    files: 0.02,
    memory: 0.06,
    cpu: 0.03,
    requests: 0.6,
    rowsRead: 0.00125,
    rowsWritten: 1.25,
    fileReads: 0.5,
    fileWrites: 6,
} as const;
