import { test, expect } from "@destack/test";
import { AuditError } from "../src/error/index.ts";
import { AuditStorage, documentRename, rename } from "./storage.ts";
import { AuditCall, defineAuditAction } from "../src/index.ts";
import { document } from "./stack/index.ts";
import { schema } from "@destack/schema";
import { AuditRecorder } from "../src/server/index.ts";
import { AccessFixture } from "@destack/access/test";
import { eq } from "@destack/db";
import { Scope } from "@destack/sync";

test("keep each call in the history of its scope", async () => {
    const storage = await AuditStorage.open();
    try {
        // record a running call of a space
        const scope = schema
            .identifier("space")
            .parse("space-01996ab0-0000-7000-8000-000000000001");
        const recorder = new AuditRecorder(
            {
                caller: { type: "system" as const, name: "integration" },
                package: documentRename.package,
                service: "document",
                scope,
            },
            storage.journal,
        );
        const recorded = recorder.begin(documentRename, rename);
        await recorder.append(recorded);
        expect(await storage.journal.deliver(storage.history)).toBe(1);

        // read it in the space's history and nowhere else
        expect([
            (await storage.history.list({ scope, limit: 100 })).items.map((record) => record.call),
            await storage.history.list({ scope: "universe", limit: 100 }),
        ]).toEqual([[recorded], { items: [], cursor: null }]);
    } finally {
        await storage.close();
    }
});

test("read the calls of a scope and of the scopes inside it by the chains their journal delivers", async () => {
    const storage = await AuditStorage.open();
    try {
        // copy an organisation with a space inside it, and another organisation
        const access = new AccessFixture(storage.database);
        const organisation = { packageId: documentRename.package.id, type: "organisation" };
        const outer = { ...organisation, scope: "universe", id: "organisation-outer" };
        const other = { ...organisation, scope: "universe", id: "organisation-other" };
        const inner = {
            packageId: documentRename.package.id,
            type: "space",
            scope: outer.id,
            id: "space-inner",
        };
        for (const scope of [outer, other, inner]) {
            await access.copyScope(scope);
        }

        // record a rename in each scope and deliver them
        const recorded = [];
        for (const scope of [outer, other, inner]) {
            const recorder = new AuditRecorder(
                {
                    caller: { type: "system" as const, name: "integration" },
                    package: documentRename.package,
                    service: "document",
                    scope: scope.id,
                },
                storage.journal,
            );
            recorded.push(
                await recorder.record(undefined, documentRename, {
                    ...rename,
                    outcome: { kind: "success" },
                }),
            );
        }
        expect(await storage.journal.deliver(storage.history)).toBe(3);

        // read the outer organisation's own calls, and its calls within with each call's enclosing scopes
        const read = async (within: boolean) =>
            (await storage.history.list({ scope: outer.id, within, limit: 10 })).items.map(
                (record) => [record.call.execution.id, record.call.execution.context.chain],
            );
        const [outerCall, , innerCall] = recorded.map((call) => call.execution.id);
        expect([await read(false), await read(true)]).toEqual([
            [[outerCall, ["universe"]]],
            [
                [outerCall, ["universe"]],
                [innerCall, ["universe", outer.id]],
            ],
        ]);
    } finally {
        await storage.close();
    }
});

test("commit and roll back application changes with their recorded calls", async () => {
    let storage = await AuditStorage.open();
    try {
        // roll back application state and recorded calls together
        const failure = new AuditError("CONFLICT", "cancel change");
        await expect(
            storage.database.transaction(async (transaction) => {
                await transaction.insert(document).values({ name: "rolled back" });
                await storage.recorder.record(transaction, documentRename, {
                    ...rename,
                    outcome: { kind: "success" },
                });
                throw failure;
            }),
        ).rejects.toBe(failure);
        expect(await storage.database.select().from(document)).toEqual([]);
        expect(await storage.journal.read()).toEqual([]);

        // keep committed changes and calls across a restart
        const call = await storage.database.transaction(async (transaction) => {
            await transaction.insert(document).values({ name: "renamed" });

            return storage.recorder.record(transaction, documentRename, {
                ...rename,
                outcome: { kind: "success" },
            });
        });
        storage = await storage.reopen();
        expect(await storage.database.select().from(document)).toEqual([{ name: "renamed" }]);
        expect(await storage.journal.read()).toEqual([call]);
    } finally {
        await storage.close();
    }
});

test("record a running call, then its outcome once, accepting repeats and refusing changes", async () => {
    let storage = await AuditStorage.open();
    try {
        // persist a prepared call only on append, with its outcome in the same record
        const running = storage.recorder.begin(documentRename, rename);
        expect(await storage.journal.read()).toEqual([]);
        await storage.recorder.append(running);
        const finished = storage.recorder.finish(running, { kind: "success" });
        await storage.recorder.append(finished);
        storage = await storage.reopen();
        await storage.recorder.append(AuditCall.parse(JSON.parse(JSON.stringify(finished))));
        expect(await storage.journal.read()).toEqual([finished]);

        // refuse a finished call with other contents
        await expect(
            storage.recorder.append({
                ...finished,
                execution: { ...finished.execution, details: { name: "different" } },
            }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: `call ${finished.execution.id} has conflicting contents`,
        });
    } finally {
        await storage.close();
    }
});

test("record a read as one access call naming what only its value tells", async () => {
    const storage = await AuditStorage.open();
    try {
        // read a document with its name from the value
        const value = await storage.recorder.read(
            documentRename,
            rename,
            async () => ({ name: "chosen" }),
            (renamed) => ({ name: renamed.name }),
        );
        const calls = await storage.journal.read();
        expect([
            value,
            calls.map((call) => [
                call.execution.category,
                call.execution.outcome,
                call.execution.details,
            ]),
        ]).toEqual([{ name: "chosen" }, [["access", { kind: "success" }, { name: "chosen" }]]]);
    } finally {
        await storage.close();
    }
});

test("leave sensitive values out of running and read details", async () => {
    const storage = await AuditStorage.open();
    try {
        // declare a sign-in with a sensitive code and an account result
        const accountSignIn = defineAuditAction(
            {
                name: "account.signIn",
                targets: documentRename.targets,
                details: schema.object({
                    method: schema.string(),
                    code: schema.sensitive(schema.string()).exactOptional(),
                    account: schema
                        .object({ id: schema.string(), secret: schema.sensitive(schema.string()) })
                        .exactOptional(),
                }),
            },
            { package: documentRename.package },
        );

        // keep the method in the running call's details
        const targets = rename.targets;
        const running = storage.recorder.begin(accountSignIn, {
            targets,
            details: { method: "code", code: "123456" },
        });
        expect(running.execution.details).toEqual({ method: "code" });

        // keep the account's identifier in the read's details
        const read = await storage.recorder.read(
            accountSignIn,
            { targets, details: { method: "code" } },
            async () => ({ id: "account-2", secret: "hunter3" }),
            (account) => ({ method: "code", account }),
        );
        const calls = await storage.journal.read();
        expect([read, calls.map((call) => call.execution.details)]).toEqual([
            { id: "account-2", secret: "hunter3" },
            [{ method: "code", account: { id: "account-2" } }],
        ]);
    } finally {
        await storage.close();
    }
});

test("keep a moved scope's earlier calls within it, and within the scope that enclosed it then", async () => {
    const storage = await AuditStorage.open();
    try {
        // copy two organisations and an account inside the first
        const access = new AccessFixture(storage.database);
        const organisation = { packageId: documentRename.package.id, type: "organisation" };
        const first = { ...organisation, scope: "universe", id: "organisation-first" };
        const second = { ...organisation, scope: "universe", id: "organisation-second" };
        const account = {
            packageId: documentRename.package.id,
            type: "account",
            scope: first.id,
            id: "account-moved",
        };
        for (const scope of [first, second, account]) {
            await access.copyScope(scope);
        }

        // record a rename in the account, move it into the second organisation, and record another
        const recorder = new AuditRecorder(
            {
                caller: { type: "system" as const, name: "integration" },
                package: documentRename.package,
                service: "document",
                scope: account.id,
            },
            storage.journal,
        );
        const record = async () => {
            const call = await recorder.record(undefined, documentRename, {
                ...rename,
                outcome: { kind: "success" },
            });
            await storage.journal.deliver(storage.history);

            return call.execution.id;
        };
        const before = await record();
        await storage.database
            .update(Scope.table)
            .set({ parent: second.id, ancestors: [second.id] })
            .where(eq(Scope.table.scope, account.id));
        const after = await record();

        // read the account within, and each organisation within
        const within = async (scope: string) =>
            (await storage.history.list({ scope, within: true, limit: 10 })).items.map(
                (item) => item.call.execution.id,
            );
        expect([await within(account.id), await within(first.id), await within(second.id)]).toEqual(
            [[before, after], [before], [after]],
        );
    } finally {
        await storage.close();
    }
});
