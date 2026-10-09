import type { EventKind } from "@destack/event/declare";
import { action } from "./action.ts";
import { log } from "./log.ts";
import { metric } from "./metric.ts";
import { span } from "./span.ts";
import { visit } from "./visit.ts";

/** The telemetry kinds every machine keeps of its own scopes: logs, spans and metric points. */
export const TELEMETRY_KINDS: readonly EventKind[] = [log, span, metric];

/** The analytics kinds a space keeps of its views: visits and actions. */
export const ANALYTICS_KINDS: readonly EventKind[] = [visit, action];

/** The kinds observability appends in a space: telemetry and analytics. */
export const OBSERVABILITY_KINDS: readonly EventKind[] = [...TELEMETRY_KINDS, ...ANALYTICS_KINDS];
