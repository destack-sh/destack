import { Duration, type schema } from "@destack/schema";
import { LogPosition } from "@destack/db";
import type { ActivityTable } from "../object/history.ts";
import { type Method } from "../method/method.ts";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { Procedure, ReplayShape, RowSchema, TargetShape } from "../method/procedure.ts";
import type { Gated, Trait } from "./trait.ts";

import { revertMethod } from "../method/tracked.ts";
/** The options of history tracking. */
export interface TrackedDefinition<Permissions extends string = string> extends Gated<Permissions> {
    /** The activity type among the object's attachments. */
    readonly activity: ObjectOf<{ table: ActivityTable }>;
    /** How long one activity continues after its last change, 10 minutes by default. */
    readonly session?: Duration;
}

/** The methods history derives. */
export type TrackedMethodMap<History> = History extends TrackedDefinition
    ? {
          readonly revert: Method<{ kind: "revert"; permission: History["by"]; mutates: true }>;
      }
    : {};

/** The procedures history derives. */
export type TrackedProcedures<Object extends ObjectType> = {
    revert: Procedure<
        schema.Object<TargetShape<Object> & ReplayShape<Object> & PositionField>,
        RowSchema<Object>
    >;
};

/** The input field naming the log position an object is read or reverted at. */
type PositionField = { readonly at: typeof LogPosition };

/** Objects keeping their history, grouped into activities. */
export const tracked: Trait<TrackedDefinition> = {
    key: "tracked",
    isDurable: true,
    options: (definition) => definition.tracked,
    columns: () => ({}),
    constraints: () => [],
    methods: (options, declared) => {
        // read history with the permission reading the object
        const reading = ["get", "list"]
            .map((kind) => Object.values(declared).find((method) => method.kind === kind))
            .find((method) => method !== undefined)?.permission;
        if (reading === undefined || reading === null) {
            throw new TypeError("an object keeping history needs a get or list method");
        }

        return { revert: revertMethod(options.by) };
    },
    validate: (options, object) => {
        // require the activity attachment and a valid session
        if (!object.attachments.some((attachment) => attachment.object === options.activity)) {
            throw new TypeError(
                `object ${object.name} keeps history but takes no ${options.activity.plural}`,
            );
        }
        if (options.session !== undefined) {
            Duration.require(options.session, `history session of ${object.name}`);
        }
    },
};
