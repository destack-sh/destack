import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { issue } from "./issue.ts";
import { nested } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A person an issue's exceptions happened to, with the last time one did. */
export const issuePerson = defineObject(
    {
        name: "issue-person",
        plural: "issuePeople",
        title: t`${plural(argument("count"), { one: "Issue person", other: "Issue people" })}`,
        scope: space,
        fields: {
            /** The person, as the exception named them. */
            person: field.string(),
            /** When an exception last happened to the person, in Unix milliseconds. */
            lastSeenAt: field.time(),
        },
        constraints: {
            person: { kind: "unique", on: ["parentId", "person"] },
        },
        access: {
            permissions: {
                read: through("parent", "read"),
                detect: through("parent", "detect"),
            },
        },
        traits: [nested({ in: issue, delete: "cascade", add: "detect" })],
    },
    (method) => ({
        /** Record the first exception that happened to a person. */
        create: method.create("detect", { visibility: "package" }),
        /** Record a later exception that happened to a person. */
        update: method.update("detect", { visibility: "package", fields: ["lastSeenAt"] }),
    }),
);
/** A person an issue affected, as its table stores it. */
export type IssuePerson = Select<typeof issuePerson.table>;
