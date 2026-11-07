import { RelationalQuery } from "@destack/object/client";
import { principal } from "@destack/access";
import {
    type Installation,
    installation,
    type InstallationSelection,
    Observation,
    type Progress,
    type Resource,
} from "@destack/object";
import { type Action, Plan, type Risk } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { bucket, database, type Rollout, rollout } from "@destack/stack/object";
import { vault } from "@destack/vault/object";
import type { Format, Style } from "../output/index.ts";
import type { Context } from "../context/context.ts";
import stackPackage from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

/** The most resources status follows, of which a stack declares a few. */
const STATUS_RESOURCES = 100;

/** The widest step risk, backward-incompatible, a plan line pads to. */
const RISK_WIDTH = 21;

/** The colour of each step risk. */
const RISK_COLOURS = {
    safe: "green",
    fallible: "yellow",
    "backward-incompatible": "yellow",
    destructive: "red",
    privileged: "magenta",
} as const satisfies Record<Risk, Format>;

/** The sign, colour and totals phrase of each step action. */
const ACTIONS = {
    create: { sign: "+", format: "green", phrase: "to create" },
    update: { sign: "~", format: "yellow", phrase: "to update" },
    delete: { sign: "-", format: "red", phrase: "to delete" },
    replace: { sign: "±", format: "red", phrase: "to replace" },
    rename: { sign: "→", format: "yellow", phrase: "to rename" },
    convert: { sign: "~", format: "yellow", phrase: "to convert" },
    restore: { sign: "+", format: "yellow", phrase: "to restore" },
    grant: { sign: "!", format: "magenta", phrase: "to grant" },
} as const satisfies Record<
    Action,
    { readonly sign: string; readonly format: Format; readonly phrase: string }
>;

/** Follow an installation of the space with its resources, and the plan it waits on. */
export async function* followInstallation(
    id: Identifier<"installation">,
    context: Context,
): AsyncGenerator<InstallationStatus> {
    // open the space's installations, rollouts and resources as the person until the follow ends
    const space = await context.requireSpace();
    const person = await context.person();
    const stack = await context.package(stackPackage, await context.locate());
    await using opened = new AsyncDisposableStack();
    const { query } = await person.open(
        { space },
        { installation, rollout, database, bucket, vault },
        opened,
    );
    const watching = new AbortController();
    opened.defer(() => watching.abort());

    // follow the installation with the resources it manages or owns
    const owned = {
        where: {
            OR: [
                { managerInstallationId: id },
                ObjectReference.columns("owner", principal.installation.reference(space, id)),
            ],
        },
        orderBy: { name: "asc" },
        limit: STATUS_RESOURCES + 1,
    } as const;
    const status = RelationalQuery.latest(
        {
            installed: query.installation.findFirst({ where: { id } }),
            rolled: query.rollout.findFirst({ where: { installationId: id } }),
            databases: query.database.findMany(owned),
            buckets: query.bucket.findMany(owned),
            vaults: query.vault.findMany(owned),
        },
        watching.signal,
    );
    for await (const { installed, rolled, databases, buckets, vaults } of status) {
        // refuse more resources than the status shows, and an installation gone
        const resources = [...databases, ...buckets, ...vaults];
        if (resources.length > STATUS_RESOURCES) {
            throw CliError.of(
                "RESOURCE_LIMIT_EXCEEDED",
                `installation ${id} has more than ${STATUS_RESOURCES} resources`,
            );
        } else if (installed === undefined || rolled === undefined) {
            throw CliError.of("INSTALLATION_MISSING", `${space} has no installation ${id}`);
        }

        // read the plan and its digest only while the installation awaits approval
        const waiting =
            Observation.progress(installed) === "approval"
                ? await stack.rollout.plan({ spaceId: space, id })
                : undefined;
        yield new InstallationStatus({
            installation: installed,
            rollout: rolled,
            resources,
            ...(waiting === undefined ? {} : { waiting }),
        });
    }
}

/** Follow an installation until current, approving each plan it waits on, refusing a failure. */
export async function driveInstallation(
    id: Identifier<"installation">,
    isApproving: boolean,
    json: boolean,
    context: Context,
): Promise<InstallationStatus> {
    // show each state live until the installation is current
    const live = context.terminal.live(json);
    let approved: string | undefined;
    for await (const status of followInstallation(id, context)) {
        await live.show(status.show(json, context));
        const { progress, waiting } = status;
        const { name } = status.installation;
        // end once current
        if (progress === "current") {
            return status;
        }
        // refuse a failure
        else if (progress === "failed") {
            throw CliError.of("INSTALLATION_FAILED", `${name} failed: ${status.message}`);
        }
        // approve a waiting plan once, when approving or once the person types yes on a terminal
        else if (waiting !== undefined && waiting.digest !== approved) {
            live.release();
            const isApproved =
                isApproving ||
                (context.environment.process.stdin.isTerminal &&
                    (await context.terminal.prompt.confirm("Approve this plan?")));
            if (!isApproved) {
                throw CliError.of(
                    "APPROVAL_REQUIRED",
                    `${name} waits on plan ${waiting.digest}: approve it with --yes, or run ${InstallationStatus.approval(status.installation, waiting.digest)}`,
                );
            }
            const { environment } = await context.native();
            const stack = await context.package(stackPackage, await context.locate());
            await stack.rollout.approve({
                spaceId: await context.requireSpace(),
                id,
                requestId: RequestId.create(environment),
                plan: waiting.digest,
            });
            approved = waiting.digest;
        }
    }

    // stop following once interrupted
    throw CliError.of("FOLLOW_STOPPED", `stopped following ${id} before it was current`);
}

/** Where an installation stands: its progress, revisions, resources and the plan it waits on. */
export class InstallationStatus {
    /** The installation. */
    readonly installation: Pick<
        Installation,
        "id" | "name" | "scope" | "selection" | "conditions" | "export" | "generation"
    >;
    /** The rollout's applied and served revisions. */
    readonly rollout: Pick<Rollout, "appliedRevisionId" | "servedRevisionId">;
    /** The resources the installation manages or owns. */
    readonly resources: readonly Pick<Resource, "name" | "conditions">[];
    /** The plan the installation waits on with its digest, absent unless it awaits approval. */
    readonly waiting: { readonly plan: Plan; readonly digest: string } | undefined;

    /** Take an installation's rollout, resources and waiting plan. */
    constructor(fields: {
        readonly installation: InstallationStatus["installation"];
        readonly rollout: InstallationStatus["rollout"];
        readonly resources: InstallationStatus["resources"];
        readonly waiting?: { readonly plan: Plan; readonly digest: string };
    }) {
        // set the installation, the rollout, the resources and the waiting plan
        this.installation = fields.installation;
        this.rollout = fields.rollout;
        this.resources = fields.resources;
        this.waiting = fields.waiting;
    }

    /** Read how far the installation is with its latest change. */
    get progress(): Progress {
        return Observation.progress(this.installation);
    }

    /** Read what the installation reports: its failure, else what its ready condition says. */
    get message(): string {
        return Observation.message(this.installation);
    }

    /** Read the reason the installation reports: failed, else its ready reason, else pending. */
    get reason(): string {
        return this.progress === "failed"
            ? "Failed"
            : (this.installation.conditions["Ready"]?.reason ?? "Pending");
    }

    /** Render the installation's progress, its resources' readiness and the plan it waits on. */
    render(format: Style): string {
        // show where the installation stands, what it follows, and its failure or wait
        const { resources, waiting, installation: installed, progress, message } = this;
        const colour = progress === "current" ? "green" : progress === "failed" ? "red" : "yellow";
        const lines = [
            `${format("bold", installed.name ?? installed.scope)}  ${format(colour, this.reason)}`,
            `  follows   ${InstallationStatus.#selection(installed.selection)}`,
            `  revision  ${this.rollout.appliedRevisionId ?? format("dim", "none applied")}`,
            ...(progress === "current" || message === "" ? [] : [`  ${format("dim", message)}`]),
        ];

        // list each resource's readiness
        if (resources.length > 0) {
            const width = Math.max(...resources.map((resource) => resource.name.length));
            lines.push("", format("bold", "resources"));
            for (const resource of resources) {
                const resourceReady = resource.conditions["Ready"]?.reason ?? "Pending";
                lines.push(
                    `  ${resource.name.padEnd(width)}  ${format(resourceReady === "Applied" ? "green" : "yellow", resourceReady)}`,
                );
            }
        }

        // show the plan waiting for approval and how to approve it
        if (waiting !== undefined && waiting.plan.steps.length > 0) {
            lines.push(
                "",
                format("bold", "waiting for approval"),
                ...InstallationStatus.renderPlan(waiting.plan, waiting.digest, format),
                format(
                    "dim",
                    `approve with: ${InstallationStatus.approval(installed, waiting.digest)}`,
                ),
            );
        }

        return lines.join("\n");
    }

    /** Write the command approving an installation's plan, naming any but the stack. */
    static approval(installed: Pick<Installation, "name" | "export">, digest: string): string {
        return installed.export === null
            ? `destack approve ${installed.name} --plan ${digest}`
            : `destack approve --plan ${digest}`;
    }

    /** Render a plan: each step with its risk and changed fields, then its totals. */
    static renderPlan(plan: Plan, digest: string, format: Style): string[] {
        // list each step under its address
        const lines: string[] = [];
        const width = Math.max(0, ...plan.steps.map((step) => step.target.length));
        for (const step of plan.steps) {
            const action = ACTIONS[step.action];
            lines.push(
                `  ${format(action.format, `${action.sign} ${step.target.padEnd(width)}`)}  ${format(RISK_COLOURS[step.risk], step.risk.padEnd(RISK_WIDTH))} ${step.detail}`,
            );
            for (const [field, { before, after }] of Object.entries(step.fields ?? {})) {
                lines.push(
                    `      ${field}: ${format("red", InstallationStatus.#value(before))} ${format("dim", "→")} ${format("green", InstallationStatus.#value(after))}`,
                );
            }
        }

        // total the steps by action and show the risk, the digest and the deferral
        const risk = Plan.classify(plan);
        const totals = Object.entries(ACTIONS).flatMap(([name, action]) => {
            const count = plan.steps.filter((step) => step.action === name).length;

            return count === 0 ? [] : [`${count} ${action.phrase}`];
        });
        lines.push(
            plan.steps.length === 0
                ? "No changes."
                : `${totals.join(", ")}, ${format(RISK_COLOURS[risk], risk)}, ${format("dim", `plan ${digest}`)}`,
            ...(plan.deferred === undefined
                ? []
                : [format("yellow", `Deferred: ${plan.deferred}`)]),
        );

        return lines;
    }

    /** Show the status as JSON or rendered text. */
    show(json: boolean, context: Context): string {
        return json ? JSON.stringify(this) : this.render(context.terminal.style);
    }

    /** Describe what an installation follows in one phrase. */
    static #selection(selection: InstallationSelection): string {
        switch (selection.kind) {
            case "release":
                return `release ${selection.version}`;
            case "tag":
                return `tag ${selection.tag}`;
            case "repository":
                return `${selection.reference.replace(/^refs\/(heads|tags)\//u, "")} of ${selection.repository}${selection.directory === "." ? "" : ` at ${selection.directory}`}`;
            case "checkout":
                return `checkout ${selection.checkout}${selection.directory === "." ? "" : ` at ${selection.directory}`}`;
        }
    }

    /** Show a field value compactly. */
    static #value(value: unknown): string {
        return value === undefined ? "absent" : JSON.stringify(value);
    }
}
