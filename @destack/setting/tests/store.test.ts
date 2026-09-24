import { expect, test } from "@destack/test";
import { RecordProvenance } from "@destack/model/source";
import { createRequestId } from "@destack/service/request";
import { reconcileSettings, getAssignment, mutateAssignment } from "../src/database/index.ts";
import { Storage } from "./fixture/storage.ts";
import { editor } from "./fixture/settings/index.ts";
import { personal, required } from "./fixture/assignment.ts";
import { applySettings, type SettingSourceOptions } from "../src/server/index.ts";
import { ServiceContext } from "@destack/service/server";
import { Caller } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { ResourceContext } from "@destack/resource/context";
import { notes } from "./fixture/settings/index.ts";
import { readAssignments, listAssignment } from "../src/database/index.ts";
import { device } from "./fixture/assignment.ts";
import { resolveSetting } from "../src/index.ts";
import { settingAssignment } from "../src/stack/index.ts";

test("share personal values, isolate device overrides and reject stale edits after recreation", async () => {
    const storage = await Storage.open();
    try {
        // keep the personal base and device override in the same authoritative database
        const input = {
            requestId: createRequestId(),
            setting: editor.reference,
            target: personal.target,
            expectedRevision: null,
            value: "vim",
        };
        const base = await mutateAssignment(storage.store, editor, input, storage.context);
        expect(await mutateAssignment(storage.store, editor, input, storage.context)).toEqual(base);
        const override = await mutateAssignment(
            storage.store,
            editor,
            {
                ...input,
                requestId: createRequestId(),
                target: device.target,
                value: "standard",
            },
            storage.context,
        );
        const records = await readAssignments(
            storage.database,
            [editor.reference],
            [personal.target, device.target],
        );
        expect(
            resolveSetting(
                editor,
                {
                    target: device.target,
                    assignments: records,
                    policies: [],
                    validUntil: null,
                },
                Date.now(),
            ),
        ).toEqual({
            setting: editor.reference,
            target: device.target,
            value: "standard",
            sources: [
                {
                    kind: "assignment",
                    id: override.id,
                    revision: override.revision,
                    target: override.target,
                },
            ],
            overridden: [
                { kind: "default", package: notes },
                { kind: "assignment", id: base.id, revision: base.revision, target: base.target },
            ],
            enforcement: "ordinary",
            validUntil: null,
        });
        expect(
            await storage.database
                .select({ scope: settingAssignment.scope, userId: settingAssignment.userId })
                .from(settingAssignment),
        ).toEqual([
            {
                scope: "user",
                userId: personal.target.kind === "user" ? personal.target.user.id : null,
            },
            {
                scope: "user",
                userId: personal.target.kind === "user" ? personal.target.user.id : null,
            },
        ]);

        // resetting removes the row and immediately exposes the personal base
        const reset = {
            ...input,
            requestId: createRequestId(),
            target: device.target,
            expectedRevision: override.revision,
        };
        expect(
            await mutateAssignment(storage.store, editor, reset, storage.context, "reset"),
        ).toBeNull();
        expect(await getAssignment(storage.database, editor.reference, device.target)).toBeNull();
        expect(await listAssignment(storage.database, device.target, 10)).toEqual({
            items: [],
            cursor: null,
        });

        // recreating an override never reuses the old concurrency token
        const replacement = await mutateAssignment(
            storage.store,
            editor,
            {
                ...input,
                requestId: createRequestId(),
                target: device.target,
            },
            storage.context,
        );
        expect(replacement.revision).not.toBe(override.revision);
        await expect(
            mutateAssignment(
                storage.store,
                editor,
                {
                    ...reset,
                    requestId: createRequestId(),
                    value: "standard",
                },
                storage.context,
            ),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "setting assignment revision has changed",
        });
        expect(
            await mutateAssignment(storage.store, editor, reset, storage.context, "reset"),
        ).toBeNull();
        expect(await getAssignment(storage.database, editor.reference, device.target)).toEqual(
            replacement,
        );

        // concurrent editors cannot both replace the same observed revision
        const competing = await Promise.allSettled(
            ["vim", "standard"].map((value) =>
                mutateAssignment(
                    storage.store,
                    editor,
                    {
                        ...input,
                        value,
                        expectedRevision: base.revision,
                        requestId: createRequestId(),
                    },
                    storage.context,
                ),
            ),
        );
        expect(competing.map((result) => result.status).sort()).toEqual(["fulfilled", "rejected"]);
        const rejected = competing.find((result) => result.status === "rejected");
        expect(rejected?.reason).toMatchObject({
            code: "CONFLICT",
            message: "setting assignment revision has changed",
        });
    } finally {
        await storage.close();
    }
}, 3000);

test("authorize source application and removal while preserving omitted collections", async () => {
    const storage = await Storage.open();
    try {
        const subject = storage.context.subject;
        const context = new ServiceContext(
            new Request("https://settings.example.test"),
            notes.id,
            "space-019f5530-8000-7000-8000-000000000003",
            new Caller({
                credential: "verified-test-session",
                audience: notes.id,
                subject,
                subjects: [subject],
                verifiedAt: Date.now(),
                expiresAt: Date.now() + 60_000,
            }),
            new ResourceContext(),
        );
        let isSourceAllowed = true;
        let isAssignmentAllowed = true;
        const options: SettingSourceOptions = {
            authorizeSource: async () => {
                if (!isSourceAllowed) {
                    throw new ServiceError("FORBIDDEN");
                }
            },
            authorize: async (_context, target) => {
                expect(target).toEqual(personal.target);
                if (!isAssignmentAllowed) {
                    throw new ServiceError("FORBIDDEN");
                }
            },
            authorizePolicy: async (_context, authority) => {
                expect(authority).toEqual(required.authority);
            },
            declarations: async () => [editor],
            audit: async () => storage.context.audit,
            policies: async () => ({ policies: [], validUntil: null }),
        };
        const { name: _name, ...source } = RecordProvenance.parse({
            kind: "stack",
            spaceId: context.scope,
            revisionId: "space-revision-019f5530-8000-7000-8000-000000000011",
            name: "editor",
        });
        const input = {
            requestId: createRequestId(),
            expectedRevision: null,
            source,
            assignments: {
                editor: { setting: editor.reference, target: personal.target, value: "vim" },
            },
        };
        const first = await applySettings(storage.store, input, notes.id, options, context);
        expect(await applySettings(storage.store, input, notes.id, options, context)).toEqual(
            first,
        );

        // adding policy alone preserves the complete existing assignment collection
        const policy = await applySettings(
            storage.store,
            {
                requestId: createRequestId(),
                expectedRevision: 1,
                source,
                policies: {
                    editor: {
                        setting: editor.reference,
                        authority: required.authority,
                        mode: "recommended",
                        value: "standard",
                    },
                },
            },
            notes.id,
            options,
            context,
        );
        expect(policy.revision).toBe(2);
        expect(policy.assignments).toEqual([]);
        expect(await getAssignment(storage.database, editor.reference, personal.target)).toEqual(
            first.assignments[0],
        );

        // removing a managed record still requires permission on its previous target
        const removal = {
            requestId: createRequestId(),
            expectedRevision: 2,
            source,
            assignments: {},
        };
        isAssignmentAllowed = false;
        await expect(
            applySettings(storage.store, removal, notes.id, options, context),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        isAssignmentAllowed = true;
        isSourceAllowed = false;
        await expect(
            applySettings(storage.store, removal, notes.id, options, context),
        ).rejects.toMatchObject({ code: "FORBIDDEN" });
        isSourceAllowed = true;
        const removed = await applySettings(storage.store, removal, notes.id, options, context);
        expect(removed.assignments).toEqual([]);
        expect(await getAssignment(storage.database, editor.reference, personal.target)).toBeNull();
        expect(removed.policies).toEqual([]);
    } finally {
        await storage.close();
    }
}, 3000);

test("reconcile complete source revisions atomically and preserve explicit detachment", async () => {
    const storage = await Storage.open();
    try {
        const provenance = RecordProvenance.parse({
            kind: "stack",
            spaceId: "space-019f5530-8000-7000-8000-000000000003",
            revisionId: "space-revision-019f5530-8000-7000-8000-000000000011",
            name: "editor",
        });
        const { name: _name, ...source } = provenance;
        const input = {
            requestId: createRequestId(),
            expectedRevision: null,
            source,
            assignments: {
                editor: { setting: editor.reference, target: personal.target, value: "vim" },
            },
            policies: {
                editor: {
                    setting: editor.reference,
                    authority: required.authority,
                    mode: "recommended" as const,
                    value: "standard",
                },
            },
        };
        const first = await reconcileSettings(storage.store, input, [editor], storage.context);
        expect(await reconcileSettings(storage.store, input, [editor], storage.context)).toEqual(
            first,
        );
        expect(first.assignments[0].provenance).toEqual(provenance);

        // direct edits require detachment; later source application leaves that choice intact
        const edit = {
            requestId: createRequestId(),
            setting: editor.reference,
            target: personal.target,
            expectedRevision: first.assignments[0].revision,
            value: "standard",
        };
        await expect(
            mutateAssignment(storage.store, editor, edit, storage.context),
        ).rejects.toMatchObject({
            code: "CONFLICT",
        });
        const detached = await mutateAssignment(
            storage.store,
            editor,
            edit,
            storage.context,
            "detach",
        );
        const second = await reconcileSettings(
            storage.store,
            {
                ...input,
                requestId: createRequestId(),
                expectedRevision: 1,
                assignments: {},
                policies: {},
            },
            [editor],
            storage.context,
        );
        expect(second.revision).toBe(2);
        expect(second.assignments).toEqual([]);
        expect(second.policies).toEqual([]);
        expect(await getAssignment(storage.database, editor.reference, personal.target)).toEqual(
            detached,
        );

        // a failed complete revision rolls back its source counter and every preceding mutation
        await expect(
            reconcileSettings(
                storage.store,
                {
                    ...input,
                    requestId: createRequestId(),
                    expectedRevision: 2,
                    policies: { editor: { ...input.policies.editor, value: 42 } },
                },
                [editor],
                storage.context,
            ),
        ).rejects.toMatchObject({ code: "BAD_REQUEST" });
        const third = await reconcileSettings(
            storage.store,
            {
                ...input,
                requestId: createRequestId(),
                expectedRevision: 2,
                assignments: {},
                policies: {},
            },
            [editor],
            storage.context,
        );
        expect(third).toEqual({ revision: 3, assignments: [], policies: [] });
    } finally {
        await storage.close();
    }
}, 3000);
