import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { plain } from "../output/index.ts";
import { Plan } from "@destack/resource";
import { InstallationStatus } from "./status.ts";

/** A space the rendered results belong to. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

test("render a plan's steps with field diffs, then its totals, risk, digest and deferral", async () => {
    const { crypto } = testSimulation().environment("cli");

    const plan: Plan = {
        steps: [
            { action: "create", target: "resource/main", risk: "safe", detail: "declare" },
            {
                action: "update",
                target: "resource/files",
                risk: "safe",
                detail: "change declared fields",
                fields: { retention: { before: "delete", after: "forever" } },
            },
            {
                action: "replace",
                target: "resource/main/table/notes__note",
                risk: "destructive",
                detail: "rebuild table: drop body",
            },
            {
                action: "delete",
                target: "network-policy/open",
                risk: "destructive",
                detail: "retire",
            },
        ],
        deferred: "binding main waits for resource main",
    };

    expect(
        InstallationStatus.renderPlan(plan, await Plan.digest(plan, crypto), plain).join("\n"),
    ).toBe(
        [
            "  + resource/main                    safe                  declare",
            "  ~ resource/files                   safe                  change declared fields",
            '      retention: "delete" → "forever"',
            "  ± resource/main/table/notes__note  destructive           rebuild table: drop body",
            "  - network-policy/open              destructive           retire",
            `1 to create, 1 to update, 1 to delete, 1 to replace, destructive, plan ${await Plan.digest(plan, crypto)}`,
            "Deferred: binding main waits for resource main",
        ].join("\n"),
    );
    expect(
        InstallationStatus.renderPlan(
            { steps: [] },
            await Plan.digest({ steps: [] }, crypto),
            plain,
        ),
    ).toEqual(["No changes."]);
});

/** Describe a stack condition observed at the first generation. */
function condition(status: "true" | "false", reason: string) {
    return { status, observedGeneration: 1, reason, message: "", lastTransitionAt: 0 };
}

test("render a stack's readiness with aligned resources and the plan it waits on", async () => {
    const { crypto } = testSimulation().environment("cli");

    const stack = {
        id: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-000000000003"),
        name: "stack",
        scope: spaceId,
        export: "stack",
        generation: 1,
        selection: { kind: "release" as const, version: "2026.9.2" },
        revisionId: schema
            .identifier("installation-revision")
            .parse("installation-revision-01996ab0-0000-7000-8000-000000000002"),
        appliedRevisionId: null,
        servedRevisionId: null,
        conditions: { Ready: condition("false", "WaitingForResources") },
    };
    const resources: InstallationStatus["resources"] = [
        { name: "main", conditions: { Ready: condition("true", "Applied") } },
        { name: "credentials", conditions: {} },
        { name: "notes", conditions: { Ready: condition("false", "AwaitingApproval") } },
    ];
    const waiting: Plan = {
        steps: [
            {
                action: "delete",
                target: "resource/notes/table/notes__draft",
                risk: "destructive",
                detail: "drop table",
            },
        ],
    };

    const digest = await Plan.digest(waiting, crypto);
    expect(
        new InstallationStatus({
            installation: stack,
            rollout: stack,
            resources,
            waiting: { plan: waiting, digest },
        }).render(plain),
    ).toBe(
        [
            "stack  WaitingForResources",
            "  follows   release 2026.9.2",
            "  revision  none applied",
            "",
            "resources",
            "  main         Applied",
            "  credentials  Pending",
            "  notes        AwaitingApproval",
            "",
            "waiting for approval",
            "  - resource/notes/table/notes__draft  destructive           drop table",
            `1 to delete, destructive, plan ${digest}`,
            `approve with: destack approve --plan ${digest}`,
        ].join("\n"),
    );
});

test("render an application waiting to be granted more access, approved by its name", async () => {
    const { crypto } = testSimulation().environment("cli");
    const revisionId = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-000000000004");
    const waiting: Plan = {
        steps: [
            {
                action: "grant",
                target: "installation/notes/access/space/package-01996ab0-0000-7000-8000-000000000005/space/read",
                risk: "privileged",
                detail: "request read on space",
            },
        ],
    };
    const digest = await Plan.digest(waiting, crypto);
    const status = new InstallationStatus({
        installation: {
            id: schema
                .identifier("installation")
                .parse("installation-01996ab0-0000-7000-8000-000000000006"),
            name: "notes",
            scope: spaceId,
            export: null,
            generation: 2,
            selection: { kind: "tag", tag: "latest" },
            conditions: {
                Ready: {
                    ...condition("false", "AwaitingApproval"),
                    observedGeneration: 2,
                    message: `approve plan ${digest} with 1 steps`,
                },
                Synced: { ...condition("true", "ReconcileSuccess"), observedGeneration: 2 },
            },
        },
        rollout: { appliedRevisionId: revisionId, servedRevisionId: null },
        resources: [],
        waiting: { plan: waiting, digest },
    });

    // wait on the approval, and show the grant with the command approving it by the installation's name
    expect({ progress: status.progress, rendered: status.render(plain) }).toEqual({
        progress: "approval",
        rendered: [
            "notes  AwaitingApproval",
            "  follows   tag latest",
            `  revision  ${revisionId}`,
            `  approve plan ${digest} with 1 steps`,
            "",
            "waiting for approval",
            "  ! installation/notes/access/space/package-01996ab0-0000-7000-8000-000000000005/space/read  privileged            request read on space",
            `1 to grant, privileged, plan ${digest}`,
            `approve with: destack approve notes --plan ${digest}`,
        ].join("\n"),
    });
});
