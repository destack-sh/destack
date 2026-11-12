import { Address, Plan, type Comparator, type Step } from "@destack/resource";
import { schema, Version, type JsonSchema } from "@destack/schema";
import { graph } from "@destack/package";
import type { ProcedureRouter } from "../procedure/index.ts";
import { describeProcedures } from "./procedure.ts";
import {
    PackageDescription,
    ProcedureDescription,
    RouterDescription,
} from "../declare/description.ts";
import type { ObjectDeclaration } from "../declare/package.ts";
import type { Package } from "../declare/package.ts";

/** The releases at which a procedure converts earlier callers' inputs. */
const CONVERSIONS = schema.jsonObject();

/** The address of a package's API in its release plans. */
const API_TARGET = "api";

/** The address of a package's mounts in its release plans. */
const MOUNT_TARGET = "mount";

/** Describe a package: the API its objects derive, its objects, mounts and views. */
export function describePackage(handle: Package): PackageDescription {
    return PackageDescription.parse({
        ...(handle.since === undefined ? {} : { since: handle.since }),
        api: describeRouter(handle.name, handle.router),
        objects: describeObjects(handle.id, handle.objects),
        mounts: Object.fromEntries(
            Object.values(handle.mounts).map((mount) => [mount.name, { path: mount.path }]),
        ),
        views: Object.keys(handle.views),
        permissions: handle.permissions,
        capabilities: handle.capabilities,
        errors: handle.errors,
    });
}

/** Describe the object types a package routes, with hooks for its declared types only. */
export function describeObjects(
    packageId: string,
    objects: Readonly<Record<string, ObjectDeclaration>>,
): Record<
    string,
    {
        readonly type: ObjectDeclaration["typeReference"];
        readonly hooks: ObjectDeclaration["hooks"];
    }
> {
    return Object.fromEntries(
        Object.entries(objects).map(([route, routed]) => [
            route,
            {
                type: routed.typeReference,
                hooks: routed.typeReference.packageId === packageId ? routed.hooks : {},
            },
        ]),
    );
}

/** Describe a router. */
export function describeRouter(name: string, router: ProcedureRouter): RouterDescription {
    return RouterDescription.parse({ name, procedures: describeProcedures(router) });
}

/** Plan a package's API and mount changes between releases. */
export const comparePackage: Comparator = (earlier, later) => {
    // read both releases
    const before = PackageDescription.parse(earlier.description);
    const after = PackageDescription.parse(later.description);
    const releases = {
        from: earlier.symbol.package.version,
        release: later.symbol.package.version,
    };

    return Plan.join([
        () => compareApi(before, after, API_TARGET, releases),
        () => compareMounts(before.mounts, after.mounts),
    ]);
};

/** Plan the mount changes between releases for the callers of earlier ones. */
function compareMounts(
    before: PackageDescription["mounts"],
    after: PackageDescription["mounts"],
): Plan {
    // plan each mount either release declares
    const names = new Set([...Object.keys(before), ...Object.keys(after)]);
    const steps = [...names].toSorted().flatMap((name): Step[] => {
        // read the mount in both releases
        const previous = before[name];
        const next = after[name];
        const target = Address.join(MOUNT_TARGET, name);

        // add a new mount
        if (previous === undefined) {
            return [{ action: "create", target, risk: "safe", detail: `add mount ${name}` }];
        }
        // remove a mount
        else if (next === undefined) {
            return [
                {
                    action: "delete",
                    target,
                    risk: "backward-incompatible",
                    detail: `remove mount ${name}`,
                },
            ];
        }
        // move a mount to another path
        else if (previous.path !== next.path) {
            return [
                {
                    action: "update",
                    target,
                    risk: "backward-incompatible",
                    detail: `move mount ${name} from ${previous.path} to ${next.path}`,
                },
            ];
        }
        // skip an unchanged mount
        else {
            return [];
        }
    });

    return { steps };
}

/** List a package's procedures as its member symbols, each served by the package. */
export function packageSymbols(handle: Package): graph.MemberSymbol[] {
    return apiSymbols(describePackage(handle).api, undefined);
}

/** Plan the changes of an API between releases for the callers of earlier ones. */
export function compareApi(
    before: Pick<PackageDescription, "since" | "api">,
    after: Pick<PackageDescription, "since" | "api">,
    target: string,
    releases: { readonly from: string; readonly release: string },
): Plan {
    // collect the plans of each change, a step each
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
            detail: `serve callers from ${after.since}: earlier callers stay on their release`,
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
        // plan the payloads for earlier callers
        else {
            plans.push(...payloadPlans(previous, procedure, at, releases));
        }
    }

    // remove the procedures the later release no longer serves
    for (const name of remaining.keys()) {
        step({
            action: "delete",
            target: Address.join(target, "procedure", name),
            risk: "backward-incompatible",
            detail: `remove procedure ${name}: earlier callers stay on their release`,
        });
    }

    return Plan.join(plans);
}

/** List an API's procedures as member symbols served by their declaration. */
export function apiSymbols(
    api: RouterDescription,
    parent: { readonly kind: string; readonly name: string } | undefined,
): graph.MemberSymbol[] {
    const procedures = api.procedures.map((procedure) => ({
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
                    ...(parent === undefined ? {} : { parent }),
                },
            })),
        },
        ...procedures.map((member) => ({ member, relationships: [] })),
    ]);
}

/** Plan a staying procedure's input for earlier callers' inputs, and its output for them. */
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
            Plan.of({
                target: Address.join(target, "input"),
                before: payload(previous.input),
                after: payload(procedure.input),
                release,
                compatibility: "backward",
                isConverted,
            }),
        () =>
            Plan.of({
                target: Address.join(target, "output"),
                before: payload(previous.output),
                after: payload(procedure.output),
                release,
                compatibility: "forward",
                isConverted: false,
            }),
    ];
}

/** Read a payload's value schema, or the event schema of a stream. */
function payload(description: ProcedureDescription["input"]): JsonSchema {
    // accept anything without a payload
    if (description === undefined) {
        return {};
    }

    return description.kind === "value" ? description.schema : description.yields;
}
