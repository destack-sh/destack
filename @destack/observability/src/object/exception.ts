import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field, installation } from "@destack/object";
import { schema } from "@destack/schema";
import { EVENT_FLUSH } from "../event/policy.ts";
import { telemetryRetention } from "../setting/index.ts";
import { IssueOpening } from "./issue.ts";
import { argument, plural, t } from "@destack/locale";

/** What an exception keeps beside its keys: its issue's opening and its revision's time. */
export const ExceptionData = schema.object({
    /** The fields opening its issue when it is the issue's first. */
    opening: IssueOpening,
    /** When the revision it ran was made, absent when unknown. */
    revisedAt: schema.number().int().nonnegative().exactOptional(),
});
/** What an exception keeps beside its keys. */
export type ExceptionData = schema.Infer<typeof ExceptionData>;

/** An unexpected exception an installation's log record or span recorded, once per identity. */
export const exception = defineObject(
    {
        name: "exception",
        plural: "exceptions",
        title: t`${plural(argument("count"), { one: "Exception", other: "Exceptions" })}`,
        scope: space,
        storage: {
            kind: "events",
            // index each issue's exceptions
            indexes: ["issue"],
            flush: EVENT_FLUSH,
            retention: { setting: telemetryRetention.definition },
            delivery: "exactly-once",
            subject: "person",
        },
        fields: {
            /** The issue it is grouped into, derived from its installation and fingerprint. */
            issue: field.string(),
            /** The installation that ran it. */
            installation: field.reference(installation),
            /** The person it happened to. */
            person: field.string().optional(),
            /** Its issue's opening and its revision's time. */
            data: field.json(ExceptionData),
        },
        access: {
            permissions: {
                read: through("scope", "observe"),
                // let the machine serving the space append what it forwards
                append: through("scope", "serve"),
            },
        },
    },
    (method) => ({
        /** Append the exceptions an export carries. */
        append: method.createMany("append", {
            visibility: "package",
            fields: ["issue", "installationId", "person", "data"],
        }),
    }),
);
