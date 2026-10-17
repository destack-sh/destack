/** The flush of every telemetry and analytics type: hourly, or at a segment of 100,000 events. */
export const TELEMETRY_FLUSH = { maxAge: { hours: 1 }, maxRows: 100_000 } as const;

/** How long telemetry stays unless a space's setting says otherwise. */
export const TELEMETRY_RETENTION = { days: 30 } as const;

/** How long analytics stay unless a space's setting says otherwise: 13 months. */
export const ANALYTICS_RETENTION = { days: 395 } as const;
