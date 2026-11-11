import { space } from "@destack/account/object";
import { principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { nested, shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The states an issue moves through. */
const ISSUE_STATES = ["backlog", "todo", "doing", "done"] as const;

/** A team of a tracker, whose members read and write its issues. */
export const team = defineObject(
    {
        name: "team",
        plural: "teams",
        title: t`${plural(argument("count"), { one: "Team", other: "Teams" })}`,
        scope: space,
        fields: {
            /** The principal who created the team. */
            owner: field.subject().caller(),
            /** The team's name. */
            name: field.string(schema.string().min(1).max(100)),
        },
        access: {
            relations: { member: { subjects: [principal.user] } },
            permissions: {
                read: union(relation("owner"), relation("member")),
                write: union(relation("owner"), relation("member")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({ create: method.create("manage", { fields: ["name"] }) }),
);

/** An issue of a team. */
export const issue = defineObject(
    {
        name: "issue",
        plural: "issues",
        title: t`${plural(argument("count"), { one: "Issue", other: "Issues" })}`,
        scope: space,
        fields: {
            /** The issue's title. */
            title: field.string(schema.string().min(1).max(200)),
            /** The issue's state. */
            state: field.enum(ISSUE_STATES).default("backlog"),
            /** The issue's priority, from 0 for none to 4 for urgent. */
            priority: field.integer().default(0),
            /** The issue's place in its column, as a fractional index. */
            position: field.fractionalIndex().default("a0"),
        },
        access: {
            permissions: { read: through("parent", "read"), write: through("parent", "write") },
        },
        traits: [nested({ in: team, delete: "cascade", add: "write" })],
    },
    (method) => ({
        create: method.create("write", { fields: ["title", "priority", "position"] }),
        update: method.update("write", { fields: ["title", "state", "priority", "position"] }),
    }),
);

/** A comment on an issue. */
export const comment = defineObject(
    {
        name: "comment",
        plural: "comments",
        title: t`${plural(argument("count"), { one: "Comment", other: "Comments" })}`,
        scope: space,
        fields: { text: field.string(schema.string().min(1).max(4000)) },
        access: {
            permissions: { read: through("parent", "read"), write: through("parent", "write") },
        },
        traits: [nested({ in: issue, delete: "cascade", add: "write" })],
    },
    (method) => ({ create: method.create("write", { fields: ["text"] }) }),
);
