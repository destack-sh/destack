import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { describeObject, objectVocabulary } from "../src/inspect/index.ts";
import { task } from "./schema.ts";

test("list the terms a task fixes in stored data: its type, relations, permissions and methods", () => {
    const json: unknown = JSON.parse(JSON.stringify(describeObject(task)));
    const description = schema.record(schema.string(), schema.json()).parse(json);
    const access = "package-01a0c80b-614e-739e-a21a-66fd749e17ca";
    const user = { packageId: access, type: "user" };
    const methods = {
        accept: "accept",
        archive: "custom",
        complete: "transition",
        create: "create",
        withdraw: "withdraw",
        export: "custom",
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
    expect(objectVocabulary(description)).toEqual({
        task: { table: "destack__object__task" },
        ...Object.fromEntries(
            Object.entries(methods).map(([name, kind]) => [`task/method/${name}`, { kind }]),
        ),
        "task/permission/plan": {},
        "task/permission/read": {},
        "task/permission/write": {},
        "task/relation/editor": { subjects: [user] },
        "task/relation/owner": { subjects: [user] },
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
