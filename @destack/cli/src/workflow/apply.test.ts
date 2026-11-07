import { testSimulation } from "@destack/object/test";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { plain } from "../output/index.ts";
import { Plan } from "@destack/resource";
import { StackStatus } from "./apply.ts";

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

    expect(StackStatus.renderPlan(plan, await Plan.digest(plan, crypto), plain).join("\n")).toBe(
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
        StackStatus.renderPlan({ steps: [] }, await Plan.digest({ steps: [] }, crypto), plain),
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
        scope: spaceId,
        selection: { kind: "release" as const, version: "2026.9.2" },
        revisionId: schema
            .identifier("installation-revision")
            .parse("installation-revision-01996ab0-0000-7000-8000-000000000002"),
        appliedRevisionId: null,
        conditions: { Ready: condition("false", "WaitingForResources") },
    };
    const resources: StackStatus["resources"] = [
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
        new StackStatus({
            installation: stack,
            rollout: stack,
            resources,
            plan: waiting,
            digest,
        }).render(plain),
    ).toBe(
        [
            `${spaceId}  WaitingForResources`,
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
