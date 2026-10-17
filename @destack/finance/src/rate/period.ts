/** A billing period, in UTC epoch milliseconds from its start up to its end. */
export interface Period {
    /** The first instant of the period. */
    readonly start: number;
    /** The first instant after the period. */
    readonly end: number;
}

/** The billing periods usage falls in. */
export const Period = {
    /** Read the UTC calendar month of a time, the period of usage without a subscription. */
    month(time: number): Period {
        const date = new Date(time);

        return {
            start: Date.UTC(date.getUTCFullYear(), date.getUTCMonth()),
            end: Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + 1),
        };
    },
};
