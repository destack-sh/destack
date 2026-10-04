import { Address, Plan, type Comparator, type Step } from "@destack/resource";
import { defineSchema, schema, Version, type JsonSchema, type JsonValue } from "@destack/schema";
import type { ServiceRouter } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";
import type { Service } from "../declare/service.ts";
import { DeclarationName, graph } from "@destack/package";

/** The releases a procedure converts earlier callers' inputs at. */
const CONVERSIONS = schema.record(schema.string(), schema.json());

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
        since: Version.exactOptional(),
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
    // list the routes the objects derive, shared ones included
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

/** Plan a service's changes between releases: its procedures, their inputs and outputs, and the earliest callers it serves. */
export const compareService: Comparator = (earlier, later) => {
    // read both releases' services and the releases declaring them
    const before = ServiceDescription.parse(earlier.description);
    const after = ServiceDescription.parse(later.description);
    const from = earlier.symbol.package.version;
    const release = later.symbol.package.version;
    const target = Address.join("service", after.name);
    const plans: (() => Plan)[] = [];
    const step = (entry: Step) => plans.push(() => ({ steps: [entry] }));

    // refuse earlier callers no longer served
    if (
        after.since !== undefined &&
        (before.since === undefined || Version.compare(after.since, before.since) > 0)
    ) {
        step({
            action: "update",
            target,
            risk: "backward-incompatible",
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
        const at = Address.join(target, "procedure", name);

        // add a new procedure
        if (previous === undefined) {
            step({ action: "create", target: at, risk: "safe", detail: `add procedure ${name}` });
        }
        // read earlier callers' inputs, and let earlier callers read the outputs
        else {
            plans.push(...payloadPlans(previous, procedure, at, { from, release }));
        }
    }

    // remove the procedures the later release no longer serves
    for (const name of remaining.keys()) {
        step({
            action: "delete",
            target: Address.join(target, "procedure", name),
            risk: "backward-incompatible",
            detail: `remove procedure ${name}: earlier callers keep their release`,
        });
    }

    return Plan.join(plans);
};

/** Plan a kept procedure's input for earlier callers' inputs and its output for earlier callers. */
function payloadPlans(
    previous: ProcedureDescription,
    procedure: ProcedureDescription,
    target: string,
    releases: { readonly from: string; readonly release: string },
): (() => Plan)[] {
    // read the releases converting earlier callers' inputs
    const { from, release } = releases;
    const declared = procedure.metadata?.["convert"];
    const convert = declared === undefined ? undefined : CONVERSIONS.parse(declared);
    const isConverted = convert !== undefined && Version.between(convert, from, release).length > 0;

    return [
        () =>
            Plan.values({
                target: Address.join(target, "input"),
                before: payload(previous.input),
                after: payload(procedure.input),
                release,
                compatibility: "backward",
                isConverted,
            }),
        () =>
            Plan.values({
                target: Address.join(target, "output"),
                before: payload(previous.output),
                after: payload(procedure.output),
                release,
                compatibility: "forward",
                isConverted: false,
            }),
    ];
}

/** Read a payload's value schema, or the event schema of a stream, accepting anything when absent. */
function payload(description: ProcedureDescription["input"]): JsonSchema {
    // accept anything without a payload
    if (description === undefined) {
        return {};
    }

    return description.kind === "value" ? description.schema : description.yields;
}

/** List a service's procedures as its member symbols, each served by the service. */
export function serviceSymbols(input: Record<string, JsonValue>): graph.MemberSymbol[] {
    const service = ServiceDescription.parse(input);
    const procedures = service.api.procedures.map((procedure) => ({
        kind: "procedure",
        name: procedure.name.join("."),
        description: procedure,
    }));

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: procedures.map((procedure) => ({
                kind: "serves",
                symbol: {
                    kind: "procedure",
                    name: procedure.name,
                    parent: { kind: "service", name: service.name },
                },
            })),
        },
        ...procedures.map((member) => ({ member, relationships: [] })),
    ]);
}
