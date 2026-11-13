import type { Environment } from "@destack/environment";
import {
    defineObject,
    field,
    HookSandbox,
    ObjectDescription,
    ObjectType,
    ObjectTypePool,
} from "@destack/object";
import { principal } from "@destack/access";
import { TABLE } from "@destack/db";
import { space } from "@destack/account/object";
import { graph, PackageVersion } from "@destack/package";
import { type Identifier, nth, schema } from "@destack/schema";
import { OBJECT_KIND, PACKAGE_KIND, Package } from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import { RequestId } from "@destack/sync";
import { testServerContext } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { ids, TestSpace } from "../src/test/index.ts";
import { shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A build declaration, as a sandbox receives it. */
type Declaration = Pick<graph.Declaration, "symbol" | "kind" | "kindPackageId" | "description">;

/** The notebooks package. */
const notebooks = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000b1",
    name: "@example/notebooks",
    version: "2026.9.0",
});

/** The tasks package. */
const tasks = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000b2",
    name: "@example/tasks",
    version: "2026.9.0",
});

/** A named record of one package, as that package declares it. */
const named = (name: string, pluralName: string) =>
    defineObject(
        {
            name,
            plural: pluralName,
            title: t`${plural(argument("count"), { one: "Record", other: "Records" })}`,
            scope: space,
            fields: { title: field.string(schema.string().min(1)) },
            traits: [shareable({})],
        },
        (method) => ({
            create: method.create("manage"),
        }),
    );

/** A listing of titled objects. */
const Listed = schema.looseObject({
    /** The listed objects. */
    items: schema.array(schema.looseObject({ title: schema.string() })),
});

/** Create a titled object of a type in the fixture's space. */
function create(type: ObjectType, title: string) {
    return type.call("create", { ...type.in(ids.space), title });
}

/** The sandbox of an installation in a world, whose types have no hooks. */
const unhooked = async (
    owner: PackageVersion,
    id: Identifier<"installation">,
    declarations: readonly Declaration[],
    environment: Environment,
) =>
    new HookSandbox({
        server: { ...(await testServerContext(owner, ids.space, environment)), installation: id },
        declarations,
        send: () => Promise.reject(new TypeError("the installation has no hooks")),
    });

/** The declarations the packages' types refer to. */
const KNOWN = [space, ...Object.values(principal)];

/** Describe a package's one type and its handle routing it, as its build's declarations do. */
function declarationsOf(owner: PackageVersion, type: ObjectType, route: string): Declaration[] {
    // describe the type as the package's own, and its handle routing it
    const description = { ...ObjectDescription.of(type), package: owner };
    const own = nth(ObjectTypePool.of(KNOWN).declare([description]), 0);
    const handle = Package.of(owner, { objects: [own] });
    const symbol = graph.Moniker.of({ packageId: owner.id, module: "src/index.ts", name: route });

    return [
        { symbol, ...OBJECT_KIND.identity, description },
        { symbol, ...PACKAGE_KIND.identity, description: describePackage(handle) },
    ];
}

test("serve two installed packages from one store, writing both in one mutation or neither", async () => {
    // serve the notebooks' and the tasks' types from the space's store over its database
    const fixture = await TestSpace.open();
    const sandboxes = [
        await unhooked(
            notebooks,
            schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-0000000000c1"),
            declarationsOf(notebooks, named("notebook", "notebooks"), "notebook"),
            fixture.machine.environment,
        ),
        await unhooked(
            tasks,
            schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-0000000000c2"),
            declarationsOf(tasks, named("task", "tasks"), "task"),
            fixture.machine.environment,
        ),
    ];
    const { store } = await fixture.spaceStore({ sandboxes });
    const types = store.objects.filter((type) =>
        [notebooks.id, tasks.id].some((id) => id === type.package.id),
    );
    await fixture.database.migrate(
        types.flatMap((type) => type.tables),
        { others: "retain" },
    );
    const [notebook, task] = [nth(types, 0), nth(types, 1)];
    const caller = store.as(notebook);

    // write a notebook and a task in one mutation
    await store.mutate(caller, {
        id: RequestId.create(fixture.machine.environment),
        calls: [create(notebook, "Work"), create(task, "Plan the week")],
    });

    // write neither when the task's call fails
    const refused = await store
        .mutate(caller, {
            id: RequestId.create(fixture.machine.environment),
            calls: [create(notebook, "Home"), create(task, "")],
        })
        .then(
            () => "written",
            () => "refused",
        );

    // read both apps' objects through the one store
    const list = async (type: ObjectType) =>
        Listed.parse(await store.query(caller, type.call("list", type.in(ids.space)))).items.map(
            (item) => item.title,
        );

    expect({
        refused,
        notebooks: await list(notebook),
        tasks: await list(task),
        tables: types.map((type) => type.table[TABLE].sqlName),
    }).toEqual({
        refused: "refused",
        notebooks: ["Work"],
        tasks: ["Plan the week"],
        tables: ["example__notebooks__notebook", "example__tasks__task"],
    });
});
