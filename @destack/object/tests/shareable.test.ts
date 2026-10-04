import { expect, test } from "@destack/test";
import { permission, principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "../src/index.ts";
import { space } from "./fixture/space.ts";

/** A board shared through the roles, containing cards. */
const board = defineObject({
    name: "board",
    plural: "boards",
    scope: space,
    fields: { title: field.string() },
    shareable: {},
    methods: (method) => ({ get: method.get("read"), update: method.update("edit") }),
});

/** Hold a permission through each of some relations. */
function holders(...relations: string[]): ReturnType<typeof relation>[] {
    return relations.map((name) => relation(name));
}

test("derive each role's permissions on an object, inherited from its parent or else from an enclosing scope shared through the roles", () => {
    // keep folders as scopes shared through the roles, containing boards, which contain cards with a verb of their own
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        scope: space,
        isScope: true,
        fields: {},
        shareable: {},
        relations: { auditor: { subjects: [principal.user] } },
        permissions: { read: relation("auditor") },
    });
    const sheet = defineObject({
        name: "sheet",
        plural: "sheets",
        scope: folder,
        fields: {},
        shareable: {},
    });
    const card = defineObject({
        name: "card",
        plural: "cards",
        scope: space,
        nested: { in: board, receive: "edit" },
        fields: { pinned: field.boolean().default(false) },
        shareable: {},
        permissions: { pin: permission("edit") },
        methods: (method) => ({ pin: method.update("pin", { fields: ["pinned"] }) }),
    });

    // derive each permission from the roles at or above it, the inherited same permission and the type's own
    const derived = (inherit: (name: string) => ReturnType<typeof through>[]) => ({
        member: union(...holders("owner", "editor", "commenter", "viewer"), ...inherit("member")),
        read: union(...holders("owner", "editor", "commenter", "viewer"), ...inherit("read")),
        comment: union(...holders("owner", "editor", "commenter"), ...inherit("comment")),
        edit: union(...holders("owner", "editor"), ...inherit("edit")),
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
        { ...derived((name) => [through("parent", name)]), pin: permission("edit") },
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
        ["owner", "pinned"],
    ]);
});

test("refuse an object shared through the roles that defines what they define", () => {
    // define an owner field holding no subject, an owner reference, an editor relation, and permission names alone
    const list = { name: "list", plural: "lists", scope: space, shareable: {} };
    expect(() => defineObject({ ...list, fields: { owner: field.string() } })).toThrow(
        "object list shares through the roles, which define owner",
    );
    expect(() =>
        defineObject({ ...list, fields: { owner: field.reference(principal.user) } }),
    ).toThrow("object list shares through the roles, which define owner");
    expect(() =>
        defineObject({
            ...list,
            fields: {},
            relations: { editor: { subjects: [principal.user] } },
        }),
    ).toThrow("object list shares through the roles, which define editor");
    expect(() => defineObject({ ...list, fields: {}, permissions: ["archive"] })).toThrow(
        "object list shares through the roles, so it declares its permissions as expressions",
    );
});
