import { PlanError } from "@destack/resource/error";
import { Plan, type Compare, type Step } from "@destack/resource";
import { defineSchema, schema, Version, type JsonSchema } from "@destack/schema";
import type { ServiceRouter } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";
import type { Service } from "../declare/service.ts";
import { DeclarationName } from "@destack/package";

/** The procedures of a service. */
export const RouterDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: schema.string().min(1),
        /** The procedures. */
        procedures: schema.array(ProcedureDescription),
    }),
);
/** The procedures of a service. */
export type RouterDescription = schema.Infer<typeof RouterDescription>;

/** A declared service. */
export const ServiceDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: DeclarationName,
        /** The oldest caller release the service serves, every release when absent. */
        since: Version.optional(),
        /** The service transport. */
        protocol: schema.literal("http"),
        /** The service's procedures. */
        api: RouterDescription,
        /** The route names of the object types the service serves. */
        objects: schema.array(schema.string().min(1)),
        /** The service's own route names beside its objects. */
        routes: schema.array(schema.string().min(1)),
    }),
);
/** A declared service. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** Describe a service. */
export function describeService(service: Service): ServiceDescription {
    // name the routes the objects derive, shared ones included
    const derived = new Set(Object.keys(service.objects));
    for (const routed of Object.values(service.objects)) {
        for (const route of Object.keys(routed.shared)) {
            derived.add(route);
        }
    }

    return ServiceDescription.parse({
        name: service.name,
        ...(service.since === undefined ? {} : { since: service.since }),
        protocol: service.protocol,
        api: describeRouter(service.name, service.router),
        objects: Object.keys(service.objects),
        routes: Object.keys(service.router).filter((route) => !derived.has(route)),
    });
}

/** Describe a router. */
export function describeRouter(name: string, service: ServiceRouter): RouterDescription {
    // describe the procedures
    const procedures = describeProcedures(service);

    return RouterDescription.parse({ name, procedures });
}

/** Plan a service's changes between releases: added and removed procedures, inputs newer servers read from earlier callers, outputs earlier callers read. */
export const compareService: Compare = (earlier, later) => {
    // read both releases' services and the releases declaring them
    const before = ServiceDescription.parse(earlier.description);
    const after = ServiceDescription.parse(later.description);
    const from = earlier.symbol.package.version;
    const release = later.symbol.package.version;

    // plan each procedure, collecting every problem the release must declare
    const steps: Step[] = [];
    const problems: PlanError["problems"][number][] = [];
    const plan = (change: Parameters<typeof Plan.schema>[0]) => {
        try {
            steps.push(...Plan.schema(change).steps);
        } catch (error) {
            // collect a missing conversion, rethrowing anything else
            if (!(error instanceof PlanError)) {
                throw error;
            }
            problems.push(...error.problems);
        }
    };

    // refuse earlier callers no longer served
    const target = `service ${after.name}`;
    if (
        after.since !== undefined &&
        (before.since === undefined || Version.compare(after.since, before.since) > 0)
    ) {
        steps.push({
            kind: "raiseSince",
            risk: "backward-incompatible",
            target,
            detail: `serve callers from ${after.since}: earlier callers keep their release`,
        });
    }

    // compare each procedure of the later release with the earlier one
    const remaining = new Map(
        before.api.procedures.map((procedure) => [procedure.name.join("."), procedure]),
    );
    for (const procedure of after.api.procedures) {
        const name = procedure.name.join(".");
        const previous = remaining.get(name);
        remaining.delete(name);
        const at = `procedure ${after.name}.${name}`;

        // add a new procedure
        if (previous === undefined) {
            steps.push({ kind: "addProcedure", risk: "safe", target: at, detail: `add ${at}` });
        }
        // read earlier callers' inputs, and let earlier callers read the outputs
        else {
            const convert = (procedure.metadata?.convert ?? {}) as Readonly<
                Record<string, unknown>
            >;
            plan({
                target: `${at} input`,
                before: payload(previous.input),
                after: payload(procedure.input),
                release,
                compatibility: "backward",
                isConverted: Version.between(Object.keys(convert), from, release).length > 0,
            });
            plan({
                target: `${at} output`,
                before: payload(previous.output),
                after: payload(procedure.output),
                release,
                compatibility: "forward",
                isConverted: false,
            });
        }
    }

    // remove the procedures the later release no longer serves
    for (const name of remaining.keys()) {
        const at = `procedure ${after.name}.${name}`;
        steps.push({
            kind: "removeProcedure",
            risk: "backward-incompatible",
            target: at,
            detail: `remove ${at}: earlier callers keep their release`,
        });
    }
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    return { steps };
};

/** Read a payload's value schema, or the event schema of a stream, accepting anything when absent. */
function payload(description: ProcedureDescription["input"]): JsonSchema {
    return description === undefined
        ? {}
        : description.kind === "value"
          ? description.schema
          : description.yields;
}
