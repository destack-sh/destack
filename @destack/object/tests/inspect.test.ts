import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { objectSymbols, objectVocabulary } from "../src/inspect/index.ts";
import { task } from "./schema.ts";
import { any } from "./fixture/match.ts";

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
        "task/permission/declare": {},
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
            description: {
                kind: "custom",
                permission: "write",
                mutates: true,
                route: { method: "POST", path: "/spaces/{spaceId}/tasks/{id}/archive" },
                input: any(Object),
            },
        },
        relationships: [
            { kind: "writes", symbol: { kind: "table", name: "destack__object__task" } },
        ],
    });
});
