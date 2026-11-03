import { principal } from "@destack/access";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { defineObject, definePackage, field, Installation } from "../src/index.ts";
import { implementPackage } from "../src/server/index.ts";
import { TestObjectStore, testSimulation } from "../src/test/index.ts";
import { notesPackage } from "./fixture/note.ts";
import { pagesPackage } from "./fixture/page.ts";
import { space } from "./fixture/space.ts";
import { users } from "./fixture/user.ts";

/** The handle of the labels package, which the store serves only once applied. */
const LABELS = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a9",
    name: "@example/labels",
    version: "2026.9.0",
});

/** A label in a space. */
const label = defineObject(
    {
        name: "label",
        plural: "labels",
        scope: space,
        fields: { name: field.string(schema.string().min(1)) },
        shareable: {},
    },
    (method) => ({ create: method.create("manage") }),
    { package: LABELS },
);

/** The package of labels. */
const labelsPackage = definePackage({ objects: { label } }, { package: LABELS });

test("serve a package applied after the store started, keeping the handlers of the packages after the first", async () => {
    // serve notes and pages over a database that stores labels too
    const simulation = testSimulation();
    const scope = space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000400");
    await using objects = await TestObjectStore.open({
        packages: [notesPackage, pagesPackage],
        tables: label.tables,
        scope,
        simulation,
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const [, pages] = objects.store.packages;

    // install labels at their base installation, then apply them beside the others
    const labels = implementPackage(labelsPackage, {}).at(
        await Installation.base(scope, labelsPackage, objects.environment.crypto),
    );
    await TestObjectStore.install(objects.store, scope, [labels]);
    await objects.apply([...objects.store.servers, labels]);

    // create a label through the applied package, pages keeping its handler
    const created = await objects
        .as(users.alice)
        .client(labelsPackage)
        .label.create({
            spaceId: objects.scope,
            requestId: RequestId.create(objects.environment),
            name: "Urgent",
        });
    const [, kept] = objects.store.packages;

    expect({
        served: objects.store.packages.map((handler) => handler.package.id),
        isKept: kept === pages,
        name: created.name,
    }).toEqual({
        served: [notesPackage.id, pagesPackage.id, labelsPackage.id],
        isKept: true,
        name: "Urgent",
    });
});
