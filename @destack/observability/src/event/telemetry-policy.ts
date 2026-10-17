import { type DatabaseConnection, Snapshot } from "@destack/db";
import type { EventKind, EventPolicy } from "@destack/event";
import { Duration, schema } from "@destack/schema";
import { SettingValue } from "@destack/setting/object";
import { ANALYTICS_TYPES, OBSERVABILITY_TYPES } from "./type.ts";
import { analyticsRetention, telemetryRetention } from "../setting/index.ts";

/** The retention of a space's telemetry and analytics, read from its settings. */
export const TelemetryPolicy = {
    /** Read a kind's policy in a scope: a space's observability settings, else its own. */
    of(database: DatabaseConnection): (kind: EventKind, scope: string) => Promise<EventPolicy> {
        return async (kind, scope) => {
            // keep the kind's own policy beyond the spaces' observability types
            const isObserved = OBSERVABILITY_TYPES.some(
                (type) => type.eventKind().key === kind.key,
            );
            if (!isObserved || !schema.identifier("space").safeParse(scope).success) {
                return kind.policy;
            }

            // keep the kind's flush, and the days the space's setting retains it
            const setting = ANALYTICS_TYPES.some((type) => type.eventKind().key === kind.key)
                ? analyticsRetention
                : telemetryRetention;
            const days = await SettingValue.resolve(Snapshot.live(database), setting, { scope });

            return { flush: kind.policy.flush, retention: Duration.milliseconds({ days }) };
        };
    },
};
