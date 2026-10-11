import type { DatabaseConnection } from "@destack/db";
import type { EventKind, EventPolicy, EventStore } from "@destack/event";
import type { Sku } from "@destack/finance";
import { usage } from "@destack/finance/declare";
import type { ObjectStore, ObjectExtension } from "@destack/object/server";
import { schema } from "@destack/schema";
import type { Controller } from "@destack/service/control";
import { SettingValue } from "@destack/setting/object";
import type { BucketHost } from "@destack/bucket/s3";
import type { OtlpReceiver } from "@destack/telemetry/otlp";
import { analytics, telemetry } from "../access/index.ts";
import { ANALYTICS_KINDS, DAY, OBSERVABILITY_KINDS, TELEMETRY_KINDS } from "../event/index.ts";
import { ObservabilityReceiver } from "../receiver/index.ts";
import { Grouper } from "../issue/group.ts";
import { MeterController } from "../meter/controller.ts";
import type { Reading } from "../meter/reading.ts";
import { observabilityService } from "../service/index.ts";
import { analyticsRetention, telemetryRetention } from "../setting/index.ts";
import { DeclarationController } from "./declaration.ts";
import { serveIssues } from "./issue.ts";
import { observabilityProcedures } from "./procedure.ts";

/** What a machine observes of the spaces it serves beside their telemetry: their builds, whose exceptions become issues, and their use of the meters. */
export interface SpaceObservation {
    /** The host keeping the spaces' build buckets, whose builds' source maps and graphs resolve exceptions into issues. */
    readonly buckets: BucketHost;
    /** Read what the spaces use over each measured interval. */
    readonly measure: (from: number, to: number) => Promise<readonly Reading[]>;
    /** How often the spaces are measured, hourly unless given. */
    readonly meters?: {
        /** How often the spaces are measured, in milliseconds. */
        readonly interval: number;
    };
    /** The SKUs rating each meter on the machine, absent on a machine billing no usage, such as a person's computer. */
    readonly skus?: readonly Sku[];
}

/** The events and spaces a space or a machine serves observability with. */
export interface ObservabilityOptions {
    /** The event store keeping the kinds the implementation lists, over the space's or the machine's database. */
    readonly events: EventStore;
    /**
     * What a machine observes of its spaces beside their telemetry.
     *
     * A machine observing only its own machine passes none: a machine runs no installations, so it keeps no issues, rules, alerts, visits or usage.
     */
    readonly spaces?: SpaceObservation;
    /** Report failures grouping exceptions. */
    readonly report: (error: unknown) => void;
}

/** The observability extension of a space or a machine, with the event kinds it appends and the receiver of its installations' and pages' OTLP exports. */
export interface ObservabilityImplementation extends ObjectExtension {
    /** The event kinds the space's or the machine's event store keeps for it. */
    readonly events: readonly EventKind[];
    /** Receive the OTLP exports of the installations and pages over the space's or machine's store, grouping a machine's exceptions into issues. */
    receiver(store: Pick<ObjectStore, "database" | "principal" | "mutate" | "clock">): OtlpReceiver;
}

/** Implement observability over a space's or a machine's events, deciding reads by the policies of the store's scopes. */
export function implementObservability(options: ObservabilityOptions): ObservabilityImplementation {
    const { events, spaces } = options;

    return {
        service: observabilityService,
        objects: spaces === undefined ? {} : serveIssues({ events }),
        policies: [telemetry, analytics],
        events:
            spaces === undefined
                ? TELEMETRY_KINDS
                : [...OBSERVABILITY_KINDS, ...(spaces.skus === undefined ? [] : [usage])],
        // route the reads, keep the declared rules and measure the spaces on a machine
        serve: (store) => ({
            procedures: observabilityProcedures(events, store.unmasked),
            controllers: controllersOf(options, store),
        }),
        receiver: (store) => {
            // group a machine's exceptions into issues and hash its visitors before appending their exports
            const receiver = new ObservabilityReceiver({
                events,
                ...(spaces === undefined
                    ? {}
                    : { grouper: new Grouper(store, spaces.buckets), salt: events.database }),
                report: options.report,
                clock: store.clock,
            });

            return { receive: (emitter, signal, body) => receiver.receive(emitter, signal, body) };
        },
    };
}

/** List the controllers of a machine's observability: the declared rules, and the meters where usage is billed. */
function controllersOf(options: ObservabilityOptions, objects: ObjectStore): Controller[] {
    const { spaces } = options;
    if (spaces === undefined) {
        return [];
    }

    return [
        new DeclarationController(objects, spaces.buckets),
        ...(spaces.skus === undefined
            ? []
            : [
                  new MeterController(options.events, spaces.measure, {
                      clock: objects.clock,
                      skus: spaces.skus,
                      ...(spaces.meters === undefined ? {} : { interval: spaces.meters.interval }),
                  }),
              ]),
    ];
}

/** The retention of a space's telemetry and analytics, read from its settings. */
export const TelemetryPolicy = {
    /** Read the policy of an observability kind in a scope from a database's settings, absent for any other kind or a scope without settings. */
    of(
        database: DatabaseConnection,
    ): (kind: EventKind, scope: string) => Promise<EventPolicy | undefined> {
        return async (kind, scope) => {
            // read the spaces' observability kinds only
            const isObserved = OBSERVABILITY_KINDS.some((each) => each.key === kind.key);
            if (!isObserved || !schema.identifier("space").safeParse(scope).success) {
                return undefined;
            }

            // keep the kind's flush, and the days the space's setting retains it
            const setting = ANALYTICS_KINDS.some((each) => each.key === kind.key)
                ? analyticsRetention
                : telemetryRetention;
            const days = await SettingValue.resolve(database, setting, { scope });

            return { flush: kind.policy.flush, retention: days * DAY };
        };
    },
};
