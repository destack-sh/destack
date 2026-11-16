import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import { archivable, defineObject, definePackage, field, nested, shareable } from "../src/index.ts";
import { TestObjectStore, testIdentity } from "../src/test/index.ts";
import { space } from "../src/test/space.ts";
import { users } from "./fixture/user.ts";
import { argument, plural, t } from "@destack/locale";

/** A project, archived out of lists once finished. */
const project = defineObject(
    {
        name: "project",
        plural: "projects",
        title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
        scope: space,
        fields: {
            name: field.string(schema.string().min(1)),
            stage: field.enum(["planned", "done"]).default("planned"),
        },
        traits: [archivable({ by: "manage" }), shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage", { fields: ["name"] }),
        finish: method.updateMany("manage", { fields: ["stage"], match: ["stage"] }),
        delete: method.delete("manage"),
    }),
);

/** A task of a project, without an archive of its own. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    traits: [nested({ in: project, delete: "cascade", add: "manage" }), shareable({})],
});

/** Projects and their tasks. */
const projectsPackage = definePackage({
    ...testIdentity,
    objects: [project, task],
});

test("archive a project out of lists and bulk changes, keep it read-only and closed to new tasks, and unarchive it", async () => {
    // serve alice's two projects
    await using objects = await TestObjectStore.open({
        packages: [projectsPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000310"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(projectsPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const launch = await client.project.create({ ...request(), name: "Launch" });
    await client.project.create({ ...request(), name: "Billing" });
    const names = async (archived: "exclude" | "include" | "only") =>
        (await client.project.list({ spaceId, archived })).items
            .map((item) => item.name)
            .toSorted((left, right) => left.localeCompare(right));

    // archive the launch, then try to rename it, add to it and archive it again
    const archived = await client.project.archive({ ...request(), id: launch.id });
    const lists = [await names("exclude"), await names("only"), await names("include")];
    const refused = [
        await refusal(client.project.update({ ...request(), id: launch.id, name: "Relaunch" })),
        await refusal(client.task.create({ ...request(), parentId: launch.id, title: "Post" })),
        await refusal(client.project.archive({ ...request(), id: launch.id })),
        await refusal(client.task.list({ spaceId, archived: "only" })),
    ];

    // finish every planned project, which leaves the archived one out
    await client.project.finish({ ...request(), where: { stage: "planned" }, stage: "done" });
    const stages = (await client.project.list({ spaceId, archived: "include" })).items
        .map((item) => [item.name, item.stage] as const)
        .toSorted((left, right) => left[0].localeCompare(right[0]));

    // unarchive it, and rename it again
    const unarchived = await client.project.unarchive({ ...request(), id: launch.id });
    const renamed = await client.project.update({ ...request(), id: launch.id, name: "Relaunch" });

    expect({
        isArchived: archived.archivedAt !== null,
        lists,
        refused,
        stages,
        unarchived: unarchived.archivedAt,
        renamed: renamed.name,
        listed: await names("exclude"),
    }).toEqual({
        isArchived: true,
        lists: [["Billing"], ["Launch"], ["Billing", "Launch"]],
        refused: [
            ["PRECONDITION_FAILED", "OBJECT_BLOCKED", "cannot change project: it is archived"],
            [
                "PRECONDITION_FAILED",
                "OBJECT_BLOCKED",
                `cannot add to project '${launch.id}': it is archived`,
            ],
            [
                "PRECONDITION_FAILED",
                "ARCHIVE_BLOCKED",
                "cannot archive project: it is already archived",
            ],
            ["BAD_REQUEST", "ARCHIVE_REQUIRED", "archive of object task is required"],
        ],
        stages: [
            ["Billing", "done"],
            ["Launch", "planned"],
        ],
        unarchived: null,
        renamed: "Relaunch",
        listed: ["Billing", "Relaunch"],
    });
});

test("keep an archived project as its tasks' parent, and delete it with them", async () => {
    // serve a task of an archived project, then delete the archived project with it
    await using objects = await TestObjectStore.open({
        packages: [projectsPackage],
        scope: space.reference(Scope.universe.id, "space-01996ab0-0000-7000-8000-000000000311"),
        owners: [principal.user.reference(Scope.universe.id, users.alice)],
    });
    const client = objects.as(users.alice).client(projectsPackage);
    const spaceId = objects.scope;
    const request = () => ({ spaceId, requestId: RequestId.create(objects.environment) });
    const launch = await client.project.create({ ...request(), name: "Launch" });
    const post = await client.task.create({ ...request(), parentId: launch.id, title: "Post" });
    await client.project.archive({ ...request(), id: launch.id });
    const related = await client.task.list({ spaceId, with: { parent: true } });
    await client.project.delete({ ...request(), id: launch.id });

    // read the parent included beside the task
    const parent = schema
        .looseObject({ name: schema.string() })
        .parse(related.included?.["parent"]?.[post.id]);
    expect({
        related: [related.items.map((item) => item.title), parent.name],
        projects: await objects.database.select().from(project.table),
        tasks: await objects.database.select().from(task.table),
    }).toEqual({
        related: [["Post"], "Launch"],
        projects: [],
        tasks: [],
    });
});
