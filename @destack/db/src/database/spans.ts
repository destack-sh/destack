import type { SpanContext, TelemetryScope } from "@destack/telemetry";

/** The transactions whose spans a database keeps: 4,096 at about 0.2 KB each, under 1 MB. */
const KEPT_TRANSACTIONS = 4096;

/** The spans this process's latest logging transactions ran in, which a pass they wake links to. */
export class TransactionSpans {
    /** The database's telemetry, which has the active span. */
    readonly #scope: TelemetryScope;
    /** The span of each kept transaction, oldest first. */
    readonly #spans = new Map<string, SpanContext>();

    /** Keep the spans of a database's transactions. */
    constructor(scope: TelemetryScope) {
        this.#scope = scope;
    }

    /** Keep the active span as a logging transaction's, forgetting the oldest transaction. */
    add(transaction: string): void {
        // leave a transaction outside any span
        const span = this.#scope.span.current();
        if (span === undefined) {
            return;
        }

        // keep it, forgetting the oldest beyond the kept ones
        this.#spans.set(transaction, span.spanContext());
        const [oldest] = this.#spans.keys();
        if (this.#spans.size > KEPT_TRANSACTIONS && oldest !== undefined) {
            this.#spans.delete(oldest);
        }
    }

    /** Read the span a transaction ran in, absent for another process's or a forgotten one. */
    get(transaction: string): SpanContext | undefined {
        return this.#spans.get(transaction);
    }
}
