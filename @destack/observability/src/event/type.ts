import type { ObjectType } from "@destack/object";
import { action } from "../object/action.ts";
import { log } from "../object/log.ts";
import { metric } from "../object/metric.ts";
import { session } from "../object/session.ts";
import { span } from "../object/span.ts";
import { visit } from "../object/visit.ts";

/** The telemetry types: logs, spans and metric points, and the sessions of a space's apps. */
export const TELEMETRY_TYPES: readonly ObjectType[] = [log, span, metric, session];

/** The analytics types a space keeps of its views: visits and actions. */
export const ANALYTICS_TYPES: readonly ObjectType[] = [visit, action];

/** The types observability appends in a space: telemetry and analytics. */
export const OBSERVABILITY_TYPES: readonly ObjectType[] = [...TELEMETRY_TYPES, ...ANALYTICS_TYPES];
