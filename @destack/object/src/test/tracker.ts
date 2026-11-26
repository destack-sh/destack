import { testIdentity } from "./index.ts";
import { schema } from "@destack/schema";
import { defineExample } from "@destack/package/declare";
import { PackageVersion } from "@destack/package";
import { archivable, defineObject, definePackage, field, numbered, shareable } from "../index.ts";
import { space } from "./space.ts";
import { argument, plural, t } from "@destack/locale";

/** The tracker package object scenarios measure numbering, archiving, keys and text edits on. */
const TRACKER = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a8",
    name: "@example/tracker",
    version: "2026.11.0",
});

/** A team addressed by its key, such as TRK, its old keys resolving after a rename. */
export const team = defineObject(
    {
        name: "team",
        plural: "teams",
        title: t`${plural(argument("count"), { one: "Team", other: "Teams" })}`,
        scope: space,
        key: { on: "key", renamed: "former" },
        fields: { key: field.string(schema.string().min(1)) },
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage", { fields: ["key"] }),
    }),
    { package: TRACKER },
);

/** An issue of a team, numbered within it as TRK-214, archived once done. */
export const issue = defineObject(
    {
        name: "issue",
        plural: "issues",
        title: t`${plural(argument("count"), { one: "Issue", other: "Issues" })}`,
        scope: space,
        fields: {
            team: field.reference(team, { delete: "restrict" }),
            title: field.string(schema.string().min(1)),
        },
        traits: [
            numbered({ within: "teamId", identifier: "live" }),
            archivable({ by: "manage" }),
            shareable({}),
        ],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage", { fields: ["teamId", "title"] }),
    }),
    { package: TRACKER },
);

/** A page whose body people write together, edit by edit. */
export const page = defineObject(
    {
        name: "page",
        plural: "pages",
        title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
        scope: space,
        fields: { title: field.string(), body: field.text() },
        traits: [shareable({})],
    },
    (method) => ({ create: method.create("manage"), update: method.update("manage") }),
    { package: TRACKER },
);

/** Teams, their issues, and pages. */
export const trackerPackage = definePackage(
    {
        ...testIdentity,
        objects: [team, issue, page],
    },
    { package: TRACKER },
);

/** The tracker package served on its own, as object scenarios are given it. */
export const trackerExample = defineExample({
    of: trackerPackage,
    name: "tracker",
    description: t`Keyed teams, their numbered and archived issues, and pages written together.`,
    render: () => trackerPackage,
});
