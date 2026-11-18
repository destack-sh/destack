import { TABLE } from "@destack/db";
import { graph } from "@destack/package";
import { Address, Plan, type Comparator } from "@destack/resource";
import { unreachablePlan } from "@destack/resource/error";
import { Edition, schema, type JsonValue } from "@destack/schema";
import { ObjectDescription } from "../object/description.ts";
import { type MethodDescription, ObjectType } from "../object/object.ts";
import { Method } from "../method/method.ts";

/** The address of an object type's methods in its release plans. */
const METHOD_TARGET = "method";

/** Describe an object type for the manifest, as the data it is declared from. */
export function describeObject(object: ObjectType): ObjectDescription {
    return ObjectDescription.of(object);
}

/** List an object type's terms: its name, relations, permissions and methods. */
export function objectVocabulary(object: ObjectType): Record<string, JsonValue> {
    // define each term by the shape stored rows depend on
    const { name } = object;
    const policy = object.policy.definition;
    const terms: Record<string, JsonValue> = { [name]: { table: object.table[TABLE].sqlName } };
    for (const [relation, declared] of Object.entries(policy.relations)) {
        terms[`${name}/relation/${relation}`] = { subjects: declared.subjects };
    }
    for (const permission of Object.keys(policy.permissions)) {
        terms[`${name}/permission/${permission}`] = {};
    }
    for (const [method, declared] of Object.entries(object.methods)) {
        if (Method.isRouted(declared)) {
            terms[`${name}/method/${method}`] = { kind: declared.kind };
        }
    }

    return terms;
}

/** List an object type's methods as member symbols that read or write the type's table. */
export function objectSymbols(object: ObjectType): graph.MemberSymbol[] {
    const table = object.table[TABLE].sqlName;

    return schema.array(graph.MemberSymbol).parse(
        Object.entries(object.describeMethods()).map(([name, described]) => ({
            member: { kind: "method", name, description: described },
            relationships: [
                {
                    kind: described.mutates ? "writes" : "reads",
                    symbol: { kind: "table", name: table },
                },
            ],
        })),
    );
}

/** Plan an object type's method changes between releases for the callers of earlier ones. */
export const compareObject: Comparator = (earlier, later) => {
    // read both releases' methods
    const before = ObjectDescription.parse(earlier.description).methods;
    const after = ObjectDescription.parse(later.description).methods;
    const from = earlier.symbol.package.version;
    const release = later.symbol.package.version;

    // add each new method, and plan each kept one's payloads for earlier callers
    const plans: (() => Plan)[] = [];
    for (const [name, method] of Object.entries(after)) {
        const previous = before[name];
        const target = Address.join(METHOD_TARGET, name);
        if (previous === undefined) {
            const step = {
                action: "create",
                target,
                risk: "safe",
                detail: `add method ${name}`,
            } as const;
            plans.push(() => ({ steps: [step] }));
        } else {
            plans.push(...payloadPlans(previous, method, target, { from, release }));
        }
    }

    // remove the methods the later release no longer serves
    for (const name of Object.keys(before).filter((each) => !Object.hasOwn(after, each))) {
        const step = {
            action: "delete",
            target: Address.join(METHOD_TARGET, name),
            risk: "backward-incompatible",
            detail: `remove method ${name}: earlier callers stay on their release`,
        } as const;
        plans.push(() => ({ steps: [step] }));
    }

    return Plan.join(plans);
};

/** Plan a kept method's input for earlier callers' inputs, and its output for them. */
function payloadPlans(
    previous: MethodDescription,
    method: MethodDescription,
    target: string,
    releases: { readonly from: string; readonly release: string },
): (() => Plan)[] {
    // read whether the later release converts earlier callers' inputs: its edition is higher
    const isConverted = Edition.of(method.convert) > Edition.of(previous.convert);

    return [
        () => {
            // refuse a conversion of an edition the earlier release has that the later one changes
            const changed = Edition.changed(previous.convert, method.convert);
            if (changed.length > 0) {
                const detail = `changes the conversion to edition ${changed.join(", ")} that release ${releases.from} has: add a new edition`;
                throw unreachablePlan([{ target: Address.join(target, "input"), detail }]);
            }

            return { steps: [] };
        },
        () =>
            Plan.of({
                target: Address.join(target, "input"),
                before: previous.input,
                after: method.input,
                release: releases.release,
                compatibility: "backward",
                isConverted,
            }),
        () =>
            Plan.of({
                target: Address.join(target, "output"),
                before: previous.output,
                after: method.output,
                release: releases.release,
                compatibility: "forward",
                isConverted: false,
            }),
    ];
}
