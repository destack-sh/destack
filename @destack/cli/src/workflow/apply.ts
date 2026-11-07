import { RelationalQuery } from "@destack/object/client";
import {
    type Installation,
    installation,
    type InstallationSelection,
    type Resource,
} from "@destack/object";
import objectPackage from "@destack/object/package";
import { Plan } from "@destack/resource";
import { type Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { bucket, database, rollout, type Rollout, StackReason } from "@destack/stack/object";
import { vault } from "@destack/vault/object";
import { type Verb, OptionValue, type Options } from "../command/command.ts";

import type { Style } from "../output/index.ts";
import type { Context } from "../context/context.ts";
import type { PlacementEndpoint } from "@destack/account/object";

import stackPackage, { STACK_INSTALLATION } from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

/** The most resources status follows, of which a stack declares a few. */
const STATUS_RESOURCES = 100;

/** The condition selecting a space's stack installation: the one exporting its stack. */
const STACK = { export: { isNotNull: true } } as const;

/** The widest step risk, backward-incompatible, a plan line pads to. */
const RISK_WIDTH = 21;

/** The colour of each step risk. */
const RISK_COLOURS = {
    safe: "green",
    fallible: "yellow",
    "backward-incompatible": "yellow",
    destructive: "red",
} as const;

/** The sign, colour and totals phrase of each step action. */
const ACTIONS = {
    create: { sign: "+", format: "green", phrase: "to create" },
    update: { sign: "~", format: "yellow", phrase: "to update" },
    delete: { sign: "-", format: "red", phrase: "to delete" },
    replace: { sign: "±", format: "red", phrase: "to replace" },
    rename: { sign: "→", format: "yellow", phrase: "to rename" },
    convert: { sign: "~", format: "yellow", phrase: "to convert" },
    restore: { sign: "+", format: "yellow", phrase: "to restore" },
} as const;

/** The options selecting the stack export and its parameters. */
const EXPORT_OPTIONS = {
    export: {
        type: "string",
        description: "The exported stack to evaluate",
        argument: "name",
    },
    parameters: {
        type: "string",
        description: "JSON arguments for a parameterised definition, - for stdin, @path for a file",
        argument: "json",
    },
} as const;

/** Apply a checkout's stack to the space and follow it until applied. */
export function applyCommand(context: Context): Verb {
    return {
        description: "Apply a checkout's stack to the space, following it until applied",
        arguments: [{ name: "directory", default: ".", isDirectory: true }],
        json: true,
        options: {
            ...EXPORT_OPTIONS,
            yes: { type: "boolean", description: "Approve each plan the stack waits on" },
        },
        async run({ arguments: { directory }, options, json }) {
            // submit the checkout's stack
            const stack = await SpaceStack.open(context);
            const id = await stack.submit(
                present(directory, "the directory argument"),
                options,
                context,
            );

            // follow it until applied, approving each waiting plan once
            const live = context.terminal.live(json);
            let approved: string | undefined;
            for await (const status of stack.follow(id, context)) {
                await live.show(status.show(json, context));
                const { digest } = status;
                // stop once the stack applied
                if (status.standing === "Applied") {
                    break;
                }
                // refuse a stack that failed to apply or is blocked, with its reason
                else if (status.standing === "ApplyFailed" || status.standing === "Blocked") {
                    const ready = present(
                        status.installation.conditions["Ready"],
                        "the stack's readiness",
                    );
                    throw CliError.of(
                        "STACK_FAILED",
                        `the stack is ${status.standing}: ${ready.message}`,
                    );
                }
                // approve a waiting plan once, with --yes or once the person types yes on a terminal
                else if (status.plan.steps.length > 0 && digest !== approved) {
                    live.release();
                    const isApproved =
                        options["yes"] === true ||
                        (context.environment.process.stdin.isTerminal &&
                            (await context.terminal.prompt.confirm("Approve this plan?")));
                    if (!isApproved) {
                        throw CliError.of(
                            "APPROVAL_REQUIRED",
                            `the stack waits on plan ${digest}: approve it with --yes, or run destack approve --plan ${digest}`,
                        );
                    }
                    await stack.approve(id, digest, context);
                    approved = digest;
                }
            }
        },
    };
}

/** Show the space's stack, its resources and the plan it waits on. */
export function planCommand(context: Context): Verb {
    return {
        description: "Show the space's applied stack, its resources and the plan it waits on",
        json: true,
        options: { watch: { type: "boolean", description: "Follow the stack as it changes" } },
        async run({ options, json }) {
            // show the stack once, or each change when watched
            const stack = await SpaceStack.open(context);
            const live = context.terminal.live(json);
            for await (const status of stack.follow(await stack.require(context), context)) {
                await live.show(status.show(json, context));
                if (options["watch"] !== true) {
                    break;
                }
            }
        },
    };
}

/** Approve the plan the space's stack waits on. */
export function approveCommand(context: Context): Verb {
    return {
        description: "Approve the plan the space's stack waits on",
        json: true,
        options: {
            plan: {
                type: "string",
                description: "The digest of the reviewed plan, as destack plan shows it",
                argument: "digest",
            },
        },
        async run({ options, json }) {
            // approve the reviewed plan's digest
            const plan = OptionValue.string(options["plan"], "plan");
            if (plan === undefined) {
                throw CliError.of("USAGE_INVALID", "give the reviewed plan's digest with --plan");
            }
            const stack = await SpaceStack.open(context);
            const approved = await stack.approve(await stack.require(context), plan, context);
            await context.terminal.print(
                json ? JSON.stringify(approved) : `Approved plan ${plan}.`,
            );
        },
    };
}

/** A space's stack: the installation exporting it, applied at the space's machine. */
export class SpaceStack {
    /** The space. */
    readonly space: Identifier<"space">;
    /** Where the space lives. */
    readonly location: PlacementEndpoint;

    /** Reach a space's stack where it lives. */
    constructor(space: Identifier<"space">, location: PlacementEndpoint) {
        this.space = space;
        this.location = location;
    }

    /** Open the stack of the space the command line acts in. */
    static async open(context: Context): Promise<SpaceStack> {
        return new SpaceStack(await context.requireSpace(), await context.locate());
    }

    /** Read the stack installation, absent before the first application. */
    async installation(context: Context): Promise<Identifier<"installation"> | undefined> {
        const objects = await context.package(objectPackage, this.location);
        const { items } = await objects.installation.list({
            scope: this.space,
            where: STACK,
            limit: 1,
        });

        return items[0]?.id;
    }

    /** Read the stack installation, refusing a space without one. */
    async require(context: Context): Promise<Identifier<"installation">> {
        const id = await this.installation(context);
        if (id === undefined) {
            throw CliError.of("STACK_MISSING", `${this.space} has no stack`);
        }

        return id;
    }

    /** Evaluate a checkout's stack and submit it, creating the stack installation when absent. */
    async submit(
        directory: string,
        values: Options,
        context: Context,
    ): Promise<Identifier<"installation">> {
        // require the export and read the parameters
        const { environment } = await context.native();
        const exported = OptionValue.string(values["export"], "export");
        if (exported === undefined) {
            throw CliError.of("USAGE_INVALID", "select the exported definition with --export");
        }
        const parameters = await OptionValue.json(
            values["parameters"],
            "parameters",
            context.environment,
            context.terminal,
        );

        // evaluate it in the innermost registered checkout containing the directory
        const registered = await context.registerCheckout(directory);
        const { packageId, submission } = await (await context.native()).machine().checkout.submit({
            machineId: registered.checkout.scope,
            id: registered.checkout.id,
            requestId: RequestId.create(environment),
            space: this.space,
            login: (await context.person()).login.row.id,
            directory: registered.path,
            export: exported,
            parameters,
        });

        // create the stack installation when absent
        const objects = await context.package(objectPackage, this.location);
        const id =
            (await this.installation(context)) ??
            (
                await objects.installation.create({
                    scope: this.space,
                    requestId: RequestId.create(environment),
                    packageId,
                    selection: submission.selection,
                    export: submission.stack.export,
                    parameters: submission.stack.parameters,
                    name: STACK_INSTALLATION,
                })
            ).id;

        // submit the revision for the stack to apply
        const stack = await context.package(stackPackage, this.location);
        await stack.rollout.submit({
            spaceId: this.space,
            id,
            requestId: RequestId.create(environment),
            ...submission,
        });

        return id;
    }

    /** Approve the plan the stack waits on, by its digest. */
    async approve(id: Identifier<"installation">, plan: string, context: Context) {
        const { environment } = await context.native();
        const stack = await context.package(stackPackage, this.location);

        return stack.rollout.approve({
            spaceId: this.space,
            id,
            requestId: RequestId.create(environment),
            plan,
        });
    }

    /** Follow the stack, its resources and the plan it waits on, read while it awaits approval. */
    async *follow(id: Identifier<"installation">, context: Context): AsyncGenerator<StackStatus> {
        // open the stack's objects as the person until the follow ends
        const person = await context.person();
        const stack = await context.package(stackPackage, this.location);
        await using opened = new AsyncDisposableStack();
        const { query } = await person.open(
            { space: this.space },
            { installation, rollout, database, bucket, vault },
            opened,
        );
        const listed = { orderBy: { createdAt: "asc" }, limit: STATUS_RESOURCES + 1 } as const;
        const watching = new AbortController();
        opened.defer(() => watching.abort());

        // yield each state, failing with the client's failure
        const status = RelationalQuery.latest(
            {
                installed: query.installation.findFirst({ where: { id } }),
                rolled: query.rollout.findFirst({ where: { installationId: id } }),
                databases: query.database.findMany(listed),
                buckets: query.bucket.findMany(listed),
                vaults: query.vault.findMany(listed),
            },
            watching.signal,
        );
        for await (const { installed, rolled, databases, buckets, vaults } of status) {
            // refuse more resources than the status shows, and a stack gone
            const resources = [...databases, ...buckets, ...vaults];
            if (resources.length > STATUS_RESOURCES) {
                throw CliError.of(
                    "RESOURCE_LIMIT_EXCEEDED",
                    `${this.space} has more than ${STATUS_RESOURCES} resources`,
                );
            } else if (installed === undefined || rolled === undefined) {
                throw CliError.of("STACK_MISSING", `${this.space} has no stack`);
            }

            // read the plan and its digest only while the stack awaits approval
            const isAwaiting = StackStatus.standing(installed, rolled) === "AwaitingApproval";
            const plan = isAwaiting
                ? await stack.rollout.plan({ spaceId: this.space, id })
                : { steps: [] };
            const digest = await Plan.digest(plan, context.environment.crypto);
            yield new StackStatus({
                installation: installed,
                rollout: rolled,
                resources,
                plan,
                digest,
            });
        }
    }
}

/** Where a space's stack stands: its installation, rollout, resources and waiting plan. */
export class StackStatus {
    /** The stack installation. */
    readonly installation: Pick<Installation, "id" | "scope" | "selection" | "conditions">;
    /** The rollout's targeted and applied revisions. */
    readonly rollout: Pick<Rollout, "revisionId" | "appliedRevisionId">;
    /** The stack's resources. */
    readonly resources: readonly Pick<Resource, "name" | "conditions">[];
    /** The plan the stack waits on, empty unless it awaits approval. */
    readonly plan: Plan;
    /** The plan's digest, which approving it names. */
    readonly digest: string;

    /** Take a stack's installation, rollout, resources and waiting plan. */
    constructor(
        fields: Pick<StackStatus, "installation" | "rollout" | "resources" | "plan" | "digest">,
    ) {
        // set the installation, the rollout, the resources and the plan
        this.installation = fields.installation;
        this.rollout = fields.rollout;
        this.resources = fields.resources;
        this.plan = fields.plan;
        this.digest = fields.digest;
    }

    /** Read why the stack stands where it does: applied, waiting, failed, or still pending. */
    get standing(): StackReason | "Pending" {
        return StackStatus.standing(this.installation, this.rollout);
    }

    /** Read why a stack stands where it does from its installation's readiness and its rollout. */
    static standing(
        installed: Pick<Installation, "conditions">,
        rolled: Pick<Rollout, "revisionId" | "appliedRevisionId">,
    ): StackReason | "Pending" {
        // the stack applied its revision
        const { revisionId, appliedRevisionId } = rolled;
        const ready = installed.conditions["Ready"];
        if (revisionId !== null && appliedRevisionId === revisionId) {
            return "Applied";
        }
        // the stack reports why it waits or failed
        else if (ready !== undefined) {
            return StackReason.parse(ready.reason);
        }
        // the stack applies its revision
        else {
            return "Pending";
        }
    }

    /** Render the stack's standing, its resources' readiness and the plan it waits on. */
    render(format: Style): string {
        // show where the stack stands
        const { resources, plan } = this;
        const reason = this.standing;
        const lines = [
            `${format("bold", this.installation.scope)}  ${format(reason === "Applied" ? "green" : "yellow", reason)}`,
            `  follows   ${StackStatus.#selection(this.installation.selection)}`,
            `  revision  ${this.rollout.appliedRevisionId ?? format("dim", "none applied")}`,
        ];

        // list each resource's readiness
        if (resources.length > 0) {
            const width = Math.max(...resources.map((resource) => resource.name.length));
            lines.push("", format("bold", "resources"));
            for (const resource of resources) {
                const ready = resource.conditions["Ready"]?.reason ?? "Pending";
                lines.push(
                    `  ${resource.name.padEnd(width)}  ${format(ready === "Applied" ? "green" : "yellow", ready)}`,
                );
            }
        }

        // show the plan waiting for approval and how to approve it
        if (plan.steps.length > 0) {
            const approval = `approve with: destack approve --plan ${this.digest}`;
            lines.push(
                "",
                format("bold", "waiting for approval"),
                ...StackStatus.renderPlan(plan, this.digest, format),
                format("dim", approval),
            );
        }

        return lines.join("\n");
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
                    `      ${field}: ${format("red", StackStatus.#value(before))} ${format("dim", "→")} ${format("green", StackStatus.#value(after))}`,
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
