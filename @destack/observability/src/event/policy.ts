/** The flush of every observability events type: hourly, or at a segment of 100,000 events. */
export const EVENT_FLUSH = { maxAge: { hours: 1 }, maxRows: 100_000 } as const;
