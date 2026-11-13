import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import {
    activity,
    defineObject,
    field,
    presentable,
    recoverable,
    searchable,
    tracked,
} from "@destack/object";
import { schema } from "@destack/schema";

/** Where a project stands. */
export const PROJECT_STATES = ["planned", "started", "paused", "completed", "canceled"] as const;

/** A body of work in a space that tasks belong to. */
export const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The project's name. */
        name: field.string(schema.string().min(1).max(200)),
        /** What the project delivers. */
        summary: field.string(schema.string().max(1000)).optional(),
        /** Where the project stands. */
        state: field.enum(PROJECT_STATES).default("planned"),
        /** The person leading the project. */
        lead: field.subject().optional(),
        /** The moment the project aims to finish by. */
        targetAt: field.time().optional(),
    },
    traits: [
        presentable({ subtitle: "summary" }),
        searchable({ text: ["name", "summary"], sort: ["name"] }),
        tracked({ by: "write", activity }),
        recoverable({ within: { days: 30 }, by: "write" }),
    ],
});
