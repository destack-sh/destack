import { mergeCompute } from "@destack/package";
import type {
    BuildExtension,
    Compilation,
    CompiledOutput,
    DeclarationModule,
    OutputDescription,
    Plugin,
} from "@destack/package/build";
import { type DeclarationReference, reference } from "@destack/package/declare";
import type { DeclarationDescription } from "@destack/package/inspect";
import { WorkloadDefinition, WorkloadDescription } from "@destack/package/workload";
import { qualify, TABLE } from "@destack/db";
import { DatabaseDeclaration } from "@destack/db/inspect";
import { JournalDescription, ServiceDescription } from "@destack/service/inspect";
import { outbox } from "@destack/service/outbox";
import { SpaceError } from "../error/index.ts";

/** The prefix of the generated modules running each workload. */
const PREFIX = "virtual:@destack/space/workload/";

/** The declaration kinds a workload handles as triggers. */
const TRIGGER_KINDS = new Set(["schedule", "webhook", "watch"]);

/** The declaration kinds a workload description references. */
const REFERENCED_KINDS = new Set([
    "resource",
    "secret",
    "service",
    "service-connection",
    ...TRIGGER_KINDS,
]);

/** Run each workload of a package in its own Bun process as its space's host starts it. */
export const spaceBuild: BuildExtension = {
    compile(compilation) {
        // emit one entry per workload
        const workloads = runnable(compilation);
        for (const workload of workloads) {
            compilation.entry(workload.entrypoint, `${PREFIX}${workload.name}`);
        }

        return [workloadPlugin(workloads)];
    },
    describe(compilation, compiled) {
        // describe each workload by the declarations its entry reaches
        const workloads: Record<string, WorkloadDescription> = {};
        for (const workload of runnable(compilation)) {
            workloads[workload.name] = describe(workload, compilation, compiled);
        }

        // refuse objects-only services the workloads leave unserved
        const served = new Set(
            Object.values(workloads).flatMap((workload) =>
                workload.services.map((service) => service.name),
            ),
        );
        const unserved = owned(compilation)
            .filter(isObjectsOnly)
            .filter((service) => !served.has(service.name));
        if (Object.keys(workloads).length > 0 && unserved.length > 0) {
            throw invalid(`no workload serves the objects-only service ${names(unserved)}`);
        }

        return { workloads } satisfies OutputDescription;
    },
};

/** A workload an output runs, with the generated module running it. */
interface Runnable {
    /** The workload's name. */
    readonly name: string;
    /** The package entrypoint running it. */
    readonly entrypoint: string;
    /** The workload's definition, as its declaration describes it. */
    readonly definition: unknown;
    /** The generated module running it. */
    readonly code: string;
}

/** List the workloads a Bun server output runs: the package's own, or one per objects-only service. */
function runnable(compilation: Compilation): Runnable[] {
    // run workloads only in Bun outputs
    if (compilation.runtime !== "bun") {
        return [];
    }

    // import every resource the package declares
    const declarations = owned(compilation);
    const resources = declarations
        .filter((declaration) => declaration.kind === "resource")
        .map((declaration) => ({
            name: declaration.name,
            module: compilation.locate(declaration),
        }));

    // run the declared workloads
    const declared = declarations.filter((declaration) => declaration.kind === "workload");
    if (declared.length > 0) {
        return declared.map((workload) => ({
            name: workload.name,
            entrypoint: `./workload/${workload.name}`,
            definition: workload.description,
            code: runModule(
                [importModule(compilation.locate(workload), "workload")],
                "workload",
                resources,
            ),
        }));
    }

    // derive one workload per objects-only service
    const services = declarations.filter(isObjectsOnly);
    if (services.length === 0) {
        return [];
    }
    const { journal, database } = storage(declarations);
    const watches = declarations.filter((declaration) => declaration.kind === "watch");
    const webhooks = declarations.filter((declaration) => declaration.kind === "webhook");

    return services.map((service) => ({
        name: service.name,
        entrypoint: `./workload/${service.name}`,
        definition: { name: service.name },
        code: runModule(
            [
                'import { AuditRecorder } from "@destack/audit";',
                'import { AuditOutbox } from "@destack/audit/outbox";',
                'import { ObjectServer } from "@destack/object/server";',
                'import { defineWorkload } from "@destack/service/workload";',
                'import { branchType } from "@destack/space/object";',
                importModule(compilation.locate(service), "service"),
                importModule(compilation.locate(database), "database"),
                importModule(compilation.locate(journal), "journal"),
                ...watches.map((watch, index) =>
                    importModule(compilation.locate(watch), `watch${index}`),
                ),
                ...webhooks.map((webhook, index) =>
                    importModule(compilation.locate(webhook), `webhook${index}`),
                ),
                "const workload = defineWorkload({",
                `    name: ${JSON.stringify(service.name)},`,
                "    start: (context) => {",
                "        const connection = database.get(context.resources);",
                "        const outbox = new AuditOutbox(connection);",
                "        const audit = AuditRecorder.service(outbox, { package: service.package, service: service.name });",
                "        const controllers = [outbox.controller(context.history)];",
                "",
                `        const watches = [${watches.map((_, index) => `watch${index}`).join(", ")}];`,
                "",
                "        const branch = Object.values(service.objects).some((object) => object.same(branchType.object)) ? { branch: branchType } : {};",
                `        return { services: [ObjectServer.serve(service, { database: connection, journal, journalKey: context.journalKey, audit, controllers, replicas: context.replicas, runs: context.runs, watches, ...branch })], webhooks: [${webhooks.map((_, index) => `webhook${index}`).join(", ")}] };`,
                "    },",
                `}, ${JSON.stringify({ package: compilation.package })});`,
            ],
            "workload",
            resources,
        ),
    }));
}

/** Select the one journal and the one database keeping it and the audit outbox. */
function storage(declarations: readonly DeclarationDescription[]): {
    journal: DeclarationDescription;
    database: DeclarationDescription;
} {
    // require one journal
    const journals = declarations.filter((declaration) => declaration.kind === "journal");
    if (journals.length !== 1) {
        throw invalid(`objects-only services need one journal, found ${names(journals) || "none"}`);
    }
    const [journal] = journals as [DeclarationDescription];

    // require one database keeping the journal and the audit outbox
    const tables = [
        qualify(journal.symbol.package, JournalDescription.parse(journal.description).name),
        outbox[TABLE].sqlName,
    ];
    const databases = declarations.filter(
        (declaration) =>
            declaration.kind === "resource" &&
            declaration.description.kind === "database" &&
            tables.every((table) => includes(declaration, table)),
    );
    if (databases.length !== 1) {
        throw invalid(
            `objects-only services need one database keeping ${tables.join(" and ")}, found ${names(databases) || "none"}`,
        );
    }

    return { journal, database: databases[0]! };
}

/** Describe a workload by the declarations its entry reaches and its compute settings. */
function describe(
    workload: Runnable,
    compilation: Compilation,
    compiled: CompiledOutput,
): WorkloadDescription {
    // select the references of each kind, implementing only the package's own services and triggers
    const selections: Record<string, DeclarationReference[]> = {
        resource: [],
        secret: [],
        service: [],
        "service-connection": [],
        trigger: [],
    };
    const seen = new Set<string>();
    for (const declaration of compiled.reach(workload.entrypoint)) {
        const owner = declaration.symbol.package;
        const isImplemented = declaration.kind === "service" || TRIGGER_KINDS.has(declaration.kind);
        if (
            !REFERENCED_KINDS.has(declaration.kind) ||
            (isImplemented && owner.id !== compilation.package.id)
        ) {
            continue;
        }

        // refuse a declaration its package declares twice
        const key = `${owner.id}:${declaration.kind}:${declaration.name}`;
        if (seen.has(key)) {
            throw invalid(`duplicate declaration: ${key}`);
        }
        seen.add(key);
        const kind = TRIGGER_KINDS.has(declaration.kind) ? "trigger" : declaration.kind;
        selections[kind]!.push(reference({ package: owner, name: declaration.name }));
    }

    return WorkloadDescription.parse({
        entrypoint: workload.entrypoint,
        services: selections.service,
        triggers: selections.trigger,
        resources: selections.resource,
        secrets: selections.secret,
        connections: selections["service-connection"],
        compute: mergeCompute(WorkloadDefinition.parse(workload.definition).compute),
    });
}

/** Resolve and load the generated module running each workload. */
function workloadPlugin(workloads: readonly Runnable[]): Plugin {
    return {
        name: "@destack/space",
        resolveId: {
            filter: { id: new RegExp(`^${RegExp.escape(PREFIX)}`) },
            handler: (id) => `\0${id}`,
        },
        load: {
            filter: { id: new RegExp(`^\\0${RegExp.escape(PREFIX)}`) },
            handler: (id) =>
                workloads.find((workload) => `\0${PREFIX}${workload.name}` === id)?.code,
        },
    };
}

/** Write a module importing its declarations, then running its workload with the package's resources. */
function runModule(
    lines: readonly string[],
    workload: string,
    resources: readonly { name: string; module: DeclarationModule }[],
): string {
    return [
        'import { run } from "@destack/space/runner";',
        ...resources.map(({ module }, index) => importModule(module, `resource${index}`)),
        ...lines,
        `await run(${workload}, { ${resources.map(({ name }, index) => `${JSON.stringify(name)}: resource${index}`).join(", ")} });`,
    ].join("\n");
}

/** Write an import of a located declaration under a local name. */
function importModule(module: DeclarationModule, local: string): string {
    return `import { ${module.export} as ${local} } from ${JSON.stringify(module.file)};`;
}

/** Select the package's own declarations. */
function owned(compilation: Compilation): DeclarationDescription[] {
    return compilation.declarations.filter(
        (declaration) => declaration.symbol.package.id === compilation.package.id,
    );
}

/** Decide whether a declaration is a service whose procedures are all object methods. */
function isObjectsOnly(declaration: DeclarationDescription): boolean {
    if (declaration.kind !== "service") {
        return false;
    }
    const { routes, objects } = ServiceDescription.parse(declaration.description);

    return routes.length === 0 && objects.length > 0;
}

/** Report whether a database declaration includes a table by its SQL name. */
function includes(database: DeclarationDescription, table: string): boolean {
    const { tables } = DatabaseDeclaration.parse(database.description);

    return tables.sqlite.some((entry) => entry.table.name === table);
}

/** Name declarations for an error. */
function names(declarations: readonly DeclarationDescription[]): string {
    return declarations.map((declaration) => declaration.symbol.symbol.name).join(", ");
}

/** Report an invalid workload. */
function invalid(message: string): SpaceError {
    return new SpaceError("INVALID_DEFINITION", message);
}
