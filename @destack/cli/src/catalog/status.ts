import {
    Installation,
    installation,
    bucket,
    database,
    edit,
    EditOutput,
} from "@destack/stack/object";
import { GitReference } from "@destack/git";
import { RelationalQuery } from "@destack/object/client";
import { principal } from "@destack/access";
import { approval, type Progress, type Resource, Status } from "@destack/object";
import { Plan, type Risk, type StepKind } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import objectPackage from "@destack/object/package";
import registryPackage from "@destack/registry/package";
import { createClient } from "@destack/object/client";
import { reference, release, tag } from "@destack/registry/object";
import { schema } from "@destack/schema";
import { ObjectReference, RequestId } from "@destack/sync";
import { vault } from "@destack/vault/object";
import type { EventState, ListState, Terminal, Tone } from "../terminal/index.ts";
import type { CommandContext } from "../command/context.ts";
import stackPackage from "@destack/stack/package";
import type {} from "@destack/package/import-meta";
import { CliError } from "../error/index.ts";

/** The most resources status follows, of which a stack declares a few. */
const STATUS_RESOURCES = 100;

/** The widest step risk, backward-incompatible, a plan line pads to. */
const RISK_WIDTH = 21;

/** The word and state of the event for each progress of an installation. */
const PROGRESS_EVENTS: Readonly<Record<Progress, readonly [string, EventState]>> = {
    current: ["Ready", "done"],
    waiting: ["Pending", "running"],
    approval: ["Pending", "running"],
    failed: ["Failed", "failed"],
};

/** How a resource stands for each progress of its record. */
const RESOURCE_STATUSES: Readonly<Record<Progress, ListState>> = {
    current: "ready",
    waiting: "busy",
    approval: "busy",
    failed: "failed",
};

/** The sign, colour and totals phrase of each step action, coloured as a diff's lines. */
const ACTIONS = {
    create: { sign: "+", tone: "done", phrase: "to create" },
    update: { sign: "~", tone: "accent", phrase: "to update" },
    delete: { sign: "-", tone: "failed", phrase: "to delete" },
    replace: { sign: "±", tone: "failed", phrase: "to replace" },
    rename: { sign: "→", tone: "accent", phrase: "to rename" },
    convert: { sign: "~", tone: "accent", phrase: "to convert" },
    restore: { sign: "+", tone: "accent", phrase: "to restore" },
    grant: { sign: "!", tone: "accent", phrase: "to grant" },
} as const satisfies Record<
    StepKind,
    { readonly sign: string; readonly tone: Tone; readonly phrase: string }
>;

/** A plan an installation waits on, with its digest and the approval its change request asks. */
interface Waiting {
    /** The plan. */
    readonly plan: Plan;
    /** The plan's digest, which approving it names. */
    readonly digest: string;
    /** The approval the plan's change request asks of the space's managers. */
    readonly approval: Identifier<"approval">;
}

/** Where an installation stands: its progress, revisions, resources and the plan it waits on. */
export class InstallationStatus {
    /** The installation. */
    readonly installation: Pick<
        Installation,
        | "id"
        | "name"
        | "scope"
        | "conditions"
        | "export"
        | "generation"
        | "targetRevisionId"
        | "appliedRevisionId"
        | "servedRevisionId"
    >;
    /** The version the installation runs, in words. */
    readonly version: string;
    /** The resources the installation manages or owns. */
    readonly resources: readonly Pick<Resource, "name" | "conditions" | "generation">[];
    /** The plan the installation waits on with its digest, absent unless it awaits approval. */
    readonly waiting: Waiting | undefined;

    /** Take an installation, its resources and waiting plan. */
    constructor(fields: {
        readonly installation: InstallationStatus["installation"];
        readonly version: string;
        readonly resources: InstallationStatus["resources"];
        readonly waiting?: Waiting;
    }) {
        // set the installation, its version, the resources and the waiting plan
        this.installation = fields.installation;
        this.version = fields.version;
        this.resources = fields.resources;
        this.waiting = fields.waiting;
    }

    /** Wait for an edit's run, then for the installation to apply a later revision. */
    static async edited(
        id: Identifier<"installation">,
        run: string,
        before: string | null,
        context: CommandContext,
    ): Promise<void> {
        // follow the run and the installation as the person
        const space = await context.requireSpace();
        const launcher = await context.launcher();
        await using opened = new AsyncDisposableStack();
        const native = await context.native();
        const { query } = await native.open(
            launcher,
            { space },
            { installation, run: edit.run },
            opened,
        );
        const watching = new AbortController();
        opened.defer(() => watching.abort());
        const states = RelationalQuery.latest(
            {
                edited: query.run.findFirst({ where: { id: schema.identifier("run").parse(run) } }),
                installed: query.installation.findFirst({ where: { id } }),
            },
            watching.signal,
        );

        // end once the run completed and the installation applies a later revision, refusing a run that did not complete
        for await (const { edited, installed } of states) {
            if (edited?.state === "failed" || edited?.state === "cancelled") {
                const reason = edited.failure?.message ?? edited.state;
                throw CliError.of(
                    "EDIT_BROKEN",
                    `edit of installation '${id}' is broken: it ended with '${reason}'`,
                );
            }
            // TODO #Broken: an edit's lease push leaves its reference row stale, so this waits
            const isApplied =
                installed !== undefined &&
                installed.appliedRevisionId !== null &&
                installed.appliedRevisionId !== before;
            const done =
                edited?.state === "completed" ? EditOutput.parse(edited.output) : undefined;
            const isUnchanged = done !== undefined && done.commit === done.base;
            if (done !== undefined && (isUnchanged || isApplied)) {
                return;
            }
        }
    }

    /** Follow an installation of the space with its resources, and the plan it waits on. */
    static async *follow(
        id: Identifier<"installation">,
        context: CommandContext,
        signal?: AbortSignal,
    ): AsyncGenerator<InstallationStatus> {
        // open the space's installations and resources as the person until the follow ends
        const space = await context.requireSpace();
        const launcher = await context.launcher();
        const stack = await context.package(stackPackage, await context.locate());
        await using opened = new AsyncDisposableStack();
        const native = await context.native();
        const { query } = await native.open(
            launcher,
            { space },
            { installation, database, bucket, vault, approval },
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
                // wake once a plan's change request asks its approval
                asked: query.approval.findMany({ where: { state: "requested" } }),
                databases: query.database.findMany(owned),
                buckets: query.bucket.findMany(owned),
                vaults: query.vault.findMany(owned),
            },
            signal === undefined ? watching.signal : AbortSignal.any([watching.signal, signal]),
        );
        let described: { readonly id: string | null; readonly version: string } | undefined;
        for await (const { installed, databases, buckets, vaults } of status) {
            // refuse more resources than the status shows, and an installation gone
            const resources = [...databases, ...buckets, ...vaults];
            if (resources.length > STATUS_RESOURCES) {
                throw CliError.of(
                    "RESOURCE_LIMIT_INVALID",
                    `installation '${id}' is invalid: it has more than ${STATUS_RESOURCES} resources`,
                );
            } else if (installed === undefined) {
                throw CliError.of(
                    "INSTALLATION_NOT_FOUND",
                    `installation '${id}' of '${space}' not found`,
                );
            }

            // read the plan, its digest and the approval it asks while the installation awaits approval
            const planned =
                Status.progress(installed) === "approval"
                    ? await stack.installation.plan({ scope: space, id })
                    : undefined;
            const waiting =
                planned === undefined || planned.approval === null
                    ? undefined
                    : { ...planned, approval: planned.approval };
            // describe its version once per version row
            if (described?.id !== installed.versionId) {
                described = {
                    id: installed.versionId,
                    version: await describeVersion(installed, context),
                };
            }
            yield new InstallationStatus({
                installation: installed,
                version: described.version,
                resources,
                ...(waiting === undefined ? {} : { waiting }),
            });
        }
    }

    /** Follow an installation being deleted until its controller removed it. */
    static async followRemoval(
        id: Identifier<"installation">,
        context: CommandContext,
    ): Promise<void> {
        // open the space's installations as the person until the follow ends
        const space = await context.requireSpace();
        const launcher = await context.launcher();
        await using opened = new AsyncDisposableStack();
        const native = await context.native();
        const { query } = await native.open(launcher, { space }, { installation }, opened);
        const watching = new AbortController();
        opened.defer(() => watching.abort());

        // end once the installation is gone
        const removal = RelationalQuery.latest(
            { installed: query.installation.findFirst({ where: { id } }) },
            watching.signal,
        );
        for await (const { installed } of removal) {
            if (installed === undefined) {
                return;
            }
        }

        // stop following once interrupted
        throw CliError.of(
            "FOLLOW_UNAVAILABLE",
            `'${id}' is unavailable: following stopped before it was removed`,
        );
    }

    /** Follow an installation until current, approving each plan it waits on, refusing failures. */
    static async drive(
        id: Identifier<"installation">,
        isApproving: boolean,
        json: boolean,
        context: CommandContext,
    ): Promise<InstallationStatus> {
        // show each state live until the installation is current and its space serves it
        const live = context.terminal.live(json);
        let approved: string | undefined;
        for await (const status of InstallationStatus.follow(id, context)) {
            await live.show(status.show(json, context));
            const { progress, waiting } = status;
            const { name } = status.installation;
            // end once current and served
            if (progress === "current" && Installation.isServed(status.installation)) {
                return status;
            }
            // refuse a failure
            else if (progress === "failed") {
                throw CliError.of(
                    "INSTALLATION_BLOCKED",
                    `cannot follow '${name}': it failed with ${status.message}`,
                );
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
                        "APPROVAL_BLOCKED",
                        `cannot install '${name}': it waits on plan '${waiting.digest}', so approve it with --yes or run '${InstallationStatus.approval(status.installation, waiting.digest)}'`,
                    );
                }
                const { environment } = await context.native();
                const objects = await context.package(objectPackage, await context.locate());
                await objects.approval.approve({
                    scope: await context.requireSpace(),
                    id: waiting.approval,
                    requestId: RequestId.create(environment),
                });
                approved = waiting.digest;
            }
        }

        // stop following once interrupted
        throw CliError.of(
            "FOLLOW_UNAVAILABLE",
            `'${id}' is unavailable: following stopped before it was current`,
        );
    }

    /** Read how far the installation is with its latest change. */
    get progress(): Progress {
        return Status.progress(this.installation);
    }

    /** Read what the installation reports: its failure, else what its ready condition says. */
    get message(): string {
        return Status.message(this.installation);
    }

    /** Render the installation's progress, its resources' readiness and the plan it waits on. */
    render(terminal: Terminal): string {
        // say where the installation stands, its version, and its failure or wait
        const { resources, waiting, installation: installed, progress, message } = this;
        const [word, state] = PROGRESS_EVENTS[progress];
        const named = `installation '${installed.name ?? installed.scope}'`;
        const said = message === "" ? named : `${named}: ${message}`;
        const lines = [
            terminal.event(word, state, said, Status.condition(installed)?.reason),
            `  ${terminal.style("dim", "version ")}  ${this.version}`,
            `  ${terminal.style("dim", "revision")}  ${this.installation.appliedRevisionId ?? "-"}`,
        ];

        // list each resource with its dot, what it reports, and the reason's code dim for searching
        if (resources.length > 0) {
            const width = Math.max(...resources.map((resource) => resource.name.length));
            lines.push("");
            for (const resource of resources) {
                const status = RESOURCE_STATUSES[Status.progress(resource)];
                const reported = Status.message(resource);
                const reason = Status.condition(resource)?.reason;
                const facts = [
                    ...(reported === "" ? [] : [reported]),
                    ...(reason === undefined ? [] : [terminal.style("dim", reason)]),
                ];
                const name = facts.length === 0 ? resource.name : resource.name.padEnd(width);
                lines.push(`  ${terminal.dot(status)} ${[name, ...facts].join("  ")}`);
            }
        }

        // show the plan waiting for approval and how to approve it
        if (waiting !== undefined && waiting.plan.steps.length > 0) {
            lines.push(
                "",
                ...InstallationStatus.renderPlan(waiting.plan, waiting.digest, terminal),
                terminal.hint("next", InstallationStatus.approval(installed, waiting.digest)),
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
    static renderPlan(plan: Plan, digest: string, terminal: Terminal): string[] {
        // list each step under its address, its sign coloured as a diff's
        const lines: string[] = [];
        const width = Math.max(0, ...plan.steps.map((step) => step.target.length));
        for (const step of plan.steps) {
            const action = ACTIONS[step.action];
            const sign = terminal.tone(action.tone, action.sign);
            const padding = " ".repeat(RISK_WIDTH - step.risk.length);
            const risk = `${InstallationStatus.#risk(step.risk, terminal)}${padding}`;
            lines.push(`  ${sign} ${step.target.padEnd(width)}  ${risk} ${step.detail}`);
            for (const [field, { before, after }] of Object.entries(step.fields ?? {})) {
                const was = terminal.style("dim", `${InstallationStatus.#value(before)} →`);
                lines.push(`      ${field}: ${was} ${InstallationStatus.#value(after)}`);
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
                ? terminal.style("dim", "  no changes")
                : `  ${terminal.style("dim", `${totals.join(", ")},`)} ${InstallationStatus.#risk(risk, terminal)}${terminal.style("dim", `, plan ${digest}`)}`,
            ...(plan.deferred === undefined ? [] : [terminal.hint("deferred", plan.deferred)]),
        );

        return lines;
    }

    /** Show the status as JSON or rendered text. */
    show(json: boolean, context: CommandContext): string {
        return json ? JSON.stringify(this) : this.render(context.terminal);
    }

    /** Write a risk: a safe one dim, one that removes or grants in the accent, any other plain. */
    static #risk(risk: Risk, terminal: Terminal): string {
        // dim what needs no look, and mark what a person must read before approving
        if (risk === "safe") {
            return terminal.style("dim", risk);
        } else if (risk === "destructive" || risk === "privileged") {
            return terminal.tone("accent", risk);
        } else {
            return risk;
        }
    }

    /** Show a field value compactly. */
    static #value(value: unknown): string {
        return value === undefined ? "absent" : JSON.stringify(value);
    }
}

/** Describe the row of an installation's version at its home, else the host's release. */
async function describeVersion(
    installed: Pick<
        Installation,
        "versionPackageId" | "versionType" | "versionScope" | "versionId" | "directory"
    >,
    context: CommandContext,
): Promise<string> {
    // describe the directory within a repository or checkout
    const at =
        installed.directory === null || installed.directory === "."
            ? ""
            : ` at ${installed.directory}`;
    const forged: readonly string[] = [tag.name, release.name, reference.name];
    const isForged = installed.versionType !== null && forged.includes(installed.versionType);

    // describe the host's release without a version row
    if (installed.versionId === null) {
        return "shipped with Destack · updates with it";
    }
    // describe a build by its identifier
    else if (!isForged) {
        return `build ${installed.versionId}${at} · pinned`;
    }

    // read the tag, release or reference at its home's forge
    const row = ObjectReference.at("version", installed);
    const home = schema.identifier("space").parse(row.scope);
    const launcher = await context.launcher();
    const endpoint = await launcher.package(home, registryPackage.id);
    const forge = createClient(registryPackage, endpoint, (await context.native()).environment);
    const read = { spaceId: home };

    // describe a tag, a release or a reference
    if (row.type === tag.name) {
        const tagged = await forge.tag.get({
            ...read,
            id: schema.identifier("tag").parse(row.id),
        });

        return `${tagged.name} · updates automatically`;
    } else if (row.type === release.name) {
        const released = await forge.release.get({
            ...read,
            id: schema.identifier("release").parse(row.id),
        });

        return `${released.version} · pinned`;
    } else {
        const referenced = await forge.reference.get({
            ...read,
            id: schema.identifier("reference").parse(row.id),
        });
        const repository = await forge.repository.get({ ...read, id: referenced.parentId });
        const branch = GitReference.short(referenced.name);

        return `${repository.name}#${branch}${at} · updates on push`;
    }
}
