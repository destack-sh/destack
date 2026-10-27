import { START, TestObservability } from "./observability.ts";

/** Five minutes before midnight, the start of tests whose issues cross into the next day. */
const BEFORE_MIDNIGHT = Date.UTC(2026, 9, 17) - 5 * 60_000;

/** The templates observability's tests copy, which the run writes once, one per start. */
export const templates = [START, BEFORE_MIDNIGHT].map((start) => TestObservability.template(start));
