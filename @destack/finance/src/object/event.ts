import { none, through } from "@destack/access";
import { account } from "@destack/account/object";
import { index, uniqueIndex, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { MeterName } from "../meter/meter.ts";

/** The longest CloudEvents id or source a meter event keeps. */
const ATTRIBUTE_LENGTH = 512;

/** The CloudEvents id of a meter event, unique among the events its source emits. */
export const MeterEventId = defineSchema(schema.string().min(1).max(ATTRIBUTE_LENGTH));

/** One occurrence of usage a meter counts for an account, kept once per CloudEvents source and id. */
export const meterEvent = defineObject({
    name: "meter-event",
    plural: "meterEvents",
    scope: account,
    fields: {
        /** The CloudEvents id, unique within its source. */
        eventId: field.string(MeterEventId),
        /** The package declaring the meter. */
        packageId: field.string(PackageId),
        /** The meter's name in its package. */
        meter: field.string(MeterName),
        /** The amount of usage, in the meter's unit. */
        value: field.number(),
        /** When the usage occurred, in UTC epoch milliseconds. */
        time: field.time(),
        /** The CloudEvents source emitting the event, such as a cell or an installation. */
        source: field.string(schema.string().min(1).max(ATTRIBUTE_LENGTH)),
    },
    permissions: {
        read: through("account", "read"),
        // TODO #Incomplete: let the account's cells and installations append meter events
        append: none(),
    },
    reserved: ["append"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("append"),
    }),
    constraints: (entry) => [
        uniqueIndex("meter_event_identity").on(entry.source, entry.eventId),
        index("meter_event_usage").on(entry.scope, entry.packageId, entry.meter, entry.time),
    ],
});
/** A persisted meter event. */
export type MeterEvent = Select<typeof meterEvent.table>;
