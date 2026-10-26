import { expect, test } from "@destack/test";
import { schema, toJsonSchema } from "@destack/schema";
import { defineObject, field } from "../src/index.ts";
import { objectSymbols, objectVocabulary } from "../src/inspect/index.ts";
import { task } from "./schema.ts";

test("list the terms a task fixes in stored data: its type, relations, permissions and methods", () => {
    const access = "package-01a0c80b-614e-739e-a21a-66fd749e17ca";
    const user = { packageId: access, type: "user" };
    const methods = {
        accept: "accept",
        archive: "custom",
        complete: "transition",
        create: "create",
        withdraw: "withdraw",
        export: "custom",
        archiveAll: "custom",
        delete: "delete",
        explain: "explain",
        get: "get",
        grant: "grant",
        list: "list",
        invitations: "invitations",
        invite: "invite",
        purge: "purge",
        relationships: "relationships",
        reopen: "transition",
        restore: "restore",
        revoke: "revoke",
        update: "update",
    };

    // key calls by the kind of each declared and derived method
    expect(objectVocabulary(task)).toEqual({
        task: { table: "destack__object__task" },
        ...Object.fromEntries(
            Object.entries(methods).map(([name, kind]) => [`task/method/${name}`, { kind }]),
        ),
        "task/permission/plan": {},
        "task/permission/read": {},
        "task/permission/tidy": {},
        "task/permission/write": {},
        "task/relation/editor": { subjects: [user] },
        "task/relation/owner": { subjects: [user] },
        "task/relation/scope": {
            subjects: [
                { packageId: "package-01a0d462-1b26-7401-be27-6258e0a32141", type: "space" },
            ],
        },
        "task/relation/viewer": {
            subjects: [
                user,
                { packageId: access, type: "installation" },
                {
                    packageId: "package-01a0d462-1b26-7401-be27-6258e0a32141",
                    type: "team",
                    relation: "member",
                },
            ],
        },
    });
});

test("list a task's methods as symbols, each with its route and input, reading or writing the task table", () => {
    const archive = objectSymbols(task).find((symbol) => symbol.member?.name === "archive");

    expect(archive).toEqual({
        member: {
            kind: "method",
            name: "archive",
            description: task.describeMethods()["archive"],
        },
        relationships: [
            { kind: "writes", symbol: { kind: "table", name: "destack__object__task" } },
        ],
    });
});

test("describe the methods callers call: kind, permission, route, and input without its request", () => {
    // define a gauge measured by a query and reset by a mutation
    const gauge = defineObject(
        {
            name: "gauge",
            plural: "gauges",
            scope: "universe",
            fields: { level: field.number() },
            permissions: ["read", "reset"],
        },
        (method) => ({
            measure: method.query({
                permission: "read",
                input: schema.object({ unit: schema.enum(["percent", "ratio"]) }),
                output: schema.object({ value: schema.number() }),
            }),
            reset: method.mutation({ permission: "reset", output: schema.object({}) }),
            tidy: method.mutation({ permission: "reset", visibility: "package" }),
        }),
    );
    const described = gauge.describeMethods();
    const { $schema: _schema, ...identifier } = toJsonSchema(gauge.idSchema);

    // describe each method but the package one, a mutation's input without its request
    expect({
        names: Object.keys(described),
        measure: described["measure"],
        reset: described["reset"],
    }).toEqual({
        names: ["measure", "reset", "get", "list"],
        measure: {
            kind: "custom",
            permission: "read",
            mutates: false,
            route: { method: "POST", path: "/gauges/{id}/measure" },
            input: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: {
                    id: identifier,
                    unit: { type: "string", enum: ["percent", "ratio"] },
                },
                required: ["id", "unit"],
                additionalProperties: false,
            },
            output: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: { value: { type: "number" } },
                required: ["value"],
                additionalProperties: false,
            },
        },
        reset: {
            kind: "custom",
            permission: "reset",
            mutates: true,
            route: { method: "POST", path: "/gauges/{id}/reset" },
            input: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: { id: identifier },
                required: ["id"],
                additionalProperties: false,
            },
            output: {
                $schema: "https://json-schema.org/draft/2020-12/schema",
                type: "object",
                properties: {},
                additionalProperties: false,
            },
        },
    });
});
