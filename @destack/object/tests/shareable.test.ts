import { expect, test } from "@destack/test";
import { permission, principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "../src/index.ts";
import { space } from "../src/test/space.ts";
import { nested, shareable } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** A board shared through the roles, containing cards. */
const board = defineObject(
    {
        name: "board",
        plural: "boards",
        title: t`${plural(argument("count"), { one: "Board", other: "Boards" })}`,
        scope: space,
        fields: { title: field.string() },
        traits: [shareable({})],
    },
    (method) => ({ update: method.update("write") }),
);

/** Grant a permission through each of some relations. */
function holders(...relations: string[]): ReturnType<typeof relation>[] {
    return relations.map((name) => relation(name));
}

test("derive each role's permissions on an object, inherited from its parent or else from an enclosing scope shared through the roles", () => {
    // declare folders as scopes shared through the roles, containing boards, which contain cards with a separate verb
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        title: t`${plural(argument("count"), { one: "Folder", other: "Folders" })}`,
        scope: space,
        fields: {},
        container: "scope",
        access: {
            relations: { auditor: { subjects: [principal.user] } },
            permissions: { read: relation("auditor") },
        },
        traits: [shareable({})],
    });
    const sheet = defineObject({
        name: "sheet",
        plural: "sheets",
        title: t`${plural(argument("count"), { one: "Sheet", other: "Sheets" })}`,
        scope: folder,
        fields: {},
        traits: [shareable({})],
    });
    const card = defineObject(
        {
            name: "card",
            plural: "cards",
            title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
            scope: space,
            fields: { starred: field.boolean().default(false) },
            access: {
                permissions: { star: permission("write") },
            },
            traits: [nested({ in: board, add: "write" }), shareable({})],
        },
        (method) => ({
            star: method.update("star", { fields: ["starred"] }),
            update: false,
        }),
    );

    // derive each permission from the roles at or above it, the same inherited permission and the type's
    const derived = (inherit: (name: string) => ReturnType<typeof through>[]) => ({
        member: union(...holders("owner", "editor", "commenter", "viewer"), ...inherit("member")),
        read: union(...holders("owner", "editor", "commenter", "viewer"), ...inherit("read")),
        comment: union(...holders("owner", "editor", "commenter"), ...inherit("comment")),
        write: union(...holders("owner", "editor"), ...inherit("write")),
        manage: union(...holders("owner"), ...inherit("manage")),
    });
    expect([
        card.policy.definition.permissions,
        sheet.policy.definition.permissions,
        sheet.policy.definition.relations["folder"],
        folder.policy.definition.permissions,
        Object.keys(folder.policy.definition.relations).toSorted(),
        Object.keys(card.fields),
    ]).toEqual([
        { ...derived((name) => [through("parent", name)]), star: permission("write") },
        derived((name) => [through("folder", name)]),
        {
            subjects: [{ packageId: folder.policy.package.id, type: "folder" }],
            isScope: true,
        },
        {
            ...derived(() => []),
            read: union(...holders("owner", "editor", "commenter", "viewer"), relation("auditor")),
        },
        ["auditor", "commenter", "editor", "owner", "viewer"],
        ["ownerPackageId", "ownerType", "ownerScope", "ownerId", "starred"],
    ]);
});
