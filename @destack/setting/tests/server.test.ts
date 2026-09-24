import { expect, test } from "@destack/test";
import { Server } from "@destack/service/server";
import { Health } from "@destack/service/health";
import { Caller } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { ResourceContext } from "@destack/resource/context";
import { createRequestId } from "@destack/service/request";
import { implementService } from "../src/server/index.ts";
import { createSettingClient, SettingClient } from "../src/client/index.ts";
import { SettingStore, mutateAssignment } from "../src/database/index.ts";
import { SETTING_BATCH_LIMIT } from "../src/service/index.ts";
import { assignmentRevision, type SettingReference } from "../src/index.ts";
import { editor, lineNumbers, notes } from "./fixture/settings/index.ts";
import { personal } from "./fixture/assignment.ts";
import { Storage } from "./fixture/storage.ts";

test("resolve typed batches and observe current values through authenticated requests", async () => {
    const storage = await Storage.open();
    const controller = new AbortController();
    try {
        // serve the real transport with explicit identity and target authorization
        let declarationReads = 0;
        let policyReads = 0;
        const policySelections: (readonly SettingReference[])[] = [];
        let requests = 0;
        let isAllowed = true;
        let validUntil: number | null = null;
        await using server = Server.start({
            ...implementService(storage.store, {
                authorize: async (_context, target, _operation, packageId) => {
                    if (
                        !isAllowed ||
                        JSON.stringify(target) !== JSON.stringify(personal.target) ||
                        (packageId !== undefined && packageId !== notes.id)
                    ) {
                        throw new ServiceError("FORBIDDEN");
                    }
                },
                authorizePolicy: async () => {
                    throw new ServiceError("FORBIDDEN");
                },
                declarations: async () => {
                    declarationReads++;

                    return [editor, lineNumbers];
                },
                policies: async (_context, references) => {
                    policyReads++;
                    policySelections.push(references);

                    return { policies: [], validUntil };
                },
                audit: async () => storage.context.audit,
            }),
            audience: notes.id,
            scope: "test-space",
            resources: new ResourceContext(),
            health: new Health("setting"),
            drainTimeout: 100,
            authenticate: async () =>
                new Caller({
                    subject: storage.context.subject,
                    subjects: [storage.context.subject],
                    credential: "test-session",
                    audience: notes.id,
                    verifiedAt: Date.now(),
                    expiresAt: Date.now() + 60_000,
                }),
            authorizeHost: async () => {},
        });
        const client = createSettingClient({
            url: "https://settings.test",
            fetch: (request) => {
                requests++;

                return server.fetch(request);
            },
        });
        const settings = new SettingClient(client, notes.id, personal.target);
        const batch = { editor, lineNumbers, duplicate: editor };

        // resolve aliases once and retain exact typed explanations under the caller's names
        const baseline = await settings.resolve(batch);
        const expected = {
            setting: editor.reference,
            target: personal.target,
            value: "standard",
            sources: [{ kind: "default", package: notes }],
            overridden: [],
            enforcement: "ordinary",
            validUntil: null,
        };
        expect(baseline).toEqual({
            editor: expected,
            lineNumbers: { ...expected, setting: lineNumbers.reference, value: true },
            duplicate: expected,
        });
        expect([requests, declarationReads, policyReads]).toEqual([1, 1, 1]);
        expect(policySelections).toEqual([[editor.reference, lineNumbers.reference]]);
        const mode: "standard" | "vim" = baseline.editor.value;
        const visible: boolean = baseline.lineNumbers.value;
        expect([mode, visible]).toEqual(["standard", true]);

        // preserve single-setting convenience through the same batched transport
        expect(await editor.get(settings)).toBe("standard");
        expect(policySelections[1]).toEqual([editor.reference]);

        // reject stale authority input for the entire batch
        validUntil = Date.now() - 1;
        await expect(settings.resolve(batch)).rejects.toMatchObject({
            code: "SERVICE_UNAVAILABLE",
        });
        validUntil = null;

        // request-scoped clients stop when their backend request is cancelled
        const request = new AbortController();
        const scoped = new SettingClient(client, notes.id, personal.target, request.signal);
        request.abort();
        await expect(scoped.resolve(batch)).rejects.toMatchObject({ name: "AbortError" });

        // deduplicate raw requests and reject incomplete or excessive selections as a whole
        const query = {
            packageId: notes.id,
            target: personal.target,
            settings: [editor.reference, lineNumbers.reference, editor.reference],
        };
        expect(await client.setting.resolve(query)).toEqual([
            baseline.editor,
            baseline.lineNumbers,
        ]);
        await expect(
            client.setting.resolve({
                ...query,
                settings: [editor.reference, { ...editor.reference, name: "missing" }],
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND" });
        await expect(client.setting.resolve({ ...query, settings: [] })).rejects.toMatchObject({
            code: "BAD_REQUEST",
        });
        await expect(
            client.setting.resolve({
                ...query,
                settings: Array(SETTING_BATCH_LIMIT + 1).fill(editor.reference),
            }),
        ).rejects.toMatchObject({ code: "BAD_REQUEST" });

        // stream one complete batch and retain notifications produced before the next read
        const watching = settings.watch(batch, controller.signal)[Symbol.asyncIterator]();
        expect(await watching.next()).toEqual({ done: false, value: baseline });
        const saved = await settings.set(editor, "vim", null);
        const changed: typeof baseline.editor = {
            ...baseline.editor,
            value: "vim",
            sources: [
                {
                    kind: "assignment",
                    id: saved.id,
                    revision: saved.revision,
                    target: saved.target,
                },
            ],
            overridden: baseline.editor.sources,
        };
        expect(await watching.next()).toEqual({
            done: false,
            value: { ...baseline, editor: changed, duplicate: changed },
        });

        // observe an independent writer by rereading current storage without a journal
        const remote = await mutateAssignment(
            new SettingStore(storage.database),
            lineNumbers,
            {
                requestId: createRequestId(),
                setting: lineNumbers.reference,
                target: personal.target,
                expectedRevision: null,
                value: false,
            },
            storage.context,
        );
        const current: typeof baseline = {
            ...baseline,
            editor: changed,
            duplicate: changed,
            lineNumbers: {
                ...baseline.lineNumbers,
                value: false,
                sources: [
                    {
                        kind: "assignment",
                        id: remote.id,
                        revision: remote.revision,
                        target: remote.target,
                    },
                ],
                overridden: baseline.lineNumbers.sources,
            },
        };
        expect(await watching.next()).toEqual({ done: false, value: current });
        expect(assignmentRevision(current.editor, personal.target)).toBe(saved.revision);
        expect(assignmentRevision(baseline.editor, personal.target)).toBeNull();

        // fail closed when access changes during an existing subscription
        isAllowed = false;
        storage.store.notify();
        await expect(watching.next()).rejects.toMatchObject({ code: "FORBIDDEN" });
        isAllowed = true;

        // reconnect from current values and release the stream on cancellation
        const reconnected = settings.watch(batch, controller.signal)[Symbol.asyncIterator]();
        expect(await reconnected.next()).toEqual({ done: false, value: current });
        controller.abort();
        await reconnected.return?.();
    } finally {
        controller.abort();
        await storage.close();
    }
}, 3000);
