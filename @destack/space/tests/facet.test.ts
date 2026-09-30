import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { defineResourceKind, type Provider } from "@destack/resource";
import { schema } from "@destack/schema";
import * as object from "../src/object/index.ts";
import { ids, openSpace, serveSpace } from "./fixture/space.ts";

/** The facet of a box resource, sharing its identity in its space. */
const box = defineObject({
    name: "box",
    identity: "resource",
    plural: "boxes",
    scope: object.space,
    fields: { label: field.string().optional() },
    permissions: ["read"],
    methods: {
        get: method.get("read"),
        create: method.create(null, { isSystem: true }),
        delete: method.delete(null, { isSystem: true }),
    },
});

/** Boxes, with nothing but their facets. */
const BoxKind = defineResourceKind("box", { spec: schema.object({}) });

/** Links, which nothing hosts. */
const LinkKind = defineResourceKind("link", { spec: schema.object({}) });

/** A provider of boxes with nothing but their facets. */
const boxes: Provider<typeof BoxKind, typeof box> = {
    kind: BoxKind,
    code: "memory",
    facet: box,
    provision: async (record) => ({ reference: `memory:${record.id}` }),
    destroy: async () => {},
};

test("create a provider's facet beside a resource it provisions, and delete it once the resource retires", async () => {
    // serve the space with the box provider, then declare a box
    const database = await openSpace([box.table]);
    await serveSpace(database, undefined, { providers: [boxes] });
    const now = Date.now();
    const [declared] = await database
        .insert(object.resource.table)
        .values({
            id: "resource-01996ab0-0000-7000-8000-00000000000e" as never,
            scope: ids.space,
            name: "box",
            kind: "box",
            definitionPackageId: ids.package,
            definitionVersion: "1.0.0",
            definitionName: "box",
            spec: {},
            retention: "delete",
            createdAt: now,
            updatedAt: now,
        })
        .returning();
    const facets = async () =>
        (await database.select({ id: box.table.id }).from(box.table)).map((row) => row.id);

    // keep the facet under the resource's identity once provisioned
    await expect.poll(facets, { timeout: 4000 }).toEqual([declared!.id]);

    // delete the facet with the resource once its deletion is requested
    await database
        .update(object.resource.table)
        .set({ deletionRequestedAt: Date.now() })
        .where(eq(object.resource.table.id, declared!.id));
    await expect.poll(facets, { timeout: 4000 }).toEqual([]);
    expect(await database.select().from(object.resource.table)).toEqual([]);
});

/** A provider of links that manages nothing: it creates, reconciles and moves nothing. */
const links: Provider<typeof LinkKind, typeof box> = { kind: LinkKind, code: "memory" };

test("refuse provisioning through a provider that manages nothing, and retire its resource without destroying", async () => {
    // serve the space with the link provider, then declare a link
    const database = await openSpace([box.table]);
    await serveSpace(database, undefined, { providers: [links] });
    const now = Date.now();
    const [declared] = await database
        .insert(object.resource.table)
        .values({
            id: "resource-01996ab0-0000-7000-8000-00000000000f" as never,
            scope: ids.space,
            name: "link",
            kind: "link",
            definitionPackageId: ids.package,
            definitionVersion: "2026.9.0",
            definitionName: "link",
            spec: {},
            retention: "delete",
            createdAt: now,
            updatedAt: now,
        })
        .returning();
    const ready = async () => {
        const [row] = await database
            .select({ conditions: object.resource.table.conditions })
            .from(object.resource.table);

        return row?.conditions.ready === undefined
            ? undefined
            : { reason: row.conditions.ready.reason, message: row.conditions.ready.message };
    };

    // observe that the provider provisions nothing
    await expect.poll(ready, { timeout: 4000 }).toEqual({
        reason: "NoProvisioning",
        message: "provider memory of link provisions nothing",
    });

    // finish the resource's deletion without destroying anything
    await database
        .update(object.resource.table)
        .set({ deletionRequestedAt: Date.now() })
        .where(eq(object.resource.table.id, declared!.id));
    await expect
        .poll(() => database.select().from(object.resource.table), { timeout: 4000 })
        .toEqual([]);
});
