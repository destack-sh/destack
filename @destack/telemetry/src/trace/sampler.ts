import { type Context, trace } from "@opentelemetry/api";
import { type Sampler, SamplingDecision, type SamplingResult } from "@opentelemetry/sdk-trace";

/** The trailing hexadecimal digits of a trace identifier the decision reads, all random. */
const DECISION_DIGITS = 8;

/** Samples a ratio of root traces, children following their parent, and records the rest for tail sampling. */
export class RatioSampler implements Sampler {
    /** The sampled ratio of root traces, from 0 to 1. */
    readonly ratio: number;

    /** Sample a ratio of root traces. */
    constructor(ratio: number) {
        this.ratio = ratio;
    }

    /** Follow a parent's decision, or sample a root whose random bits fall below the ratio. */
    shouldSample(context: Context, traceId: string): SamplingResult {
        // follow the parent's decision, recording an unsampled child for tail sampling
        const parent = trace.getSpanContext(context);
        if (parent !== undefined) {
            const isSampled = (parent.traceFlags & 1) === 1;

            return {
                decision: isSampled ? SamplingDecision.RECORD_AND_SAMPLED : SamplingDecision.RECORD,
            };
        }

        // sample a root whose random bits fall below the ratio
        const bits = Number.parseInt(traceId.slice(-DECISION_DIGITS), 16);
        const isSampled = bits < this.ratio * 16 ** DECISION_DIGITS;

        return {
            decision: isSampled ? SamplingDecision.RECORD_AND_SAMPLED : SamplingDecision.RECORD,
        };
    }

    /** Describe the sampler. */
    toString(): string {
        return `RatioSampler{${this.ratio}}`;
    }
}
