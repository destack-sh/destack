import { Condition, type Moment } from "@destack/db";
import { PackageError, PackageId } from "@destack/package";
import { type JsonObject, JsonValue, schema } from "@destack/schema";
import { ObjectReference, type ObjectTypeReference } from "@destack/sync";
import { ObjectError, refuseInput } from "../error/error.ts";
import { Field } from "../field/field.ts";
import type { ObjectType, PermissionOf } from "../object/object.ts";
import type { ConditionOf } from "../query/index.ts";
import {
    type ActionInputs,
    type ActionTemplate,
    type ActionValues,
    creating,
    type ValuesOf,
    Variable,
} from "./variable.ts";
import { ActionInput, ActionInputDescription } from "./input.ts";
import { type ActionContext, Template } from "./template.ts";

/** The input field of the object a call acts on. */
const TARGET_FIELD = "id";

/** The name of the input with the objects a method's own action acts on. */
const TARGET_INPUT = "target";

/** An action's input as far as it resolves, and the inputs left to ask for. */
export interface ActionResolution {
    /** The inputs' values so far. */
    readonly values: ActionValues;
    /** The values a created object starts with, beyond what the action sets. */
    readonly initial: JsonObject;
    /** The inputs the person still gives. */
    readonly asked: readonly ActionInput[];
}

/** One call of a method an action makes, in the scope of the object it acts on. */
export interface ActionInvocation {
    /** The scope, absent for the one the action runs in. */
    readonly scope: string | undefined;
    /** The method's input. */
    readonly input: JsonObject;
}

/** The values an action runs with, and what a created object starts with beyond what it sets. */
export interface ActionArguments<Inputs extends ActionInputs = ActionInputs> {
    /** The inputs' values. */
    readonly values: ValuesOf<Inputs>;
    /** What a created object starts with. */
    readonly initial?: JsonObject;
}

/** What a side gives a call to run: its context, and how it reads an object's row. */
export interface ActionRunner {
    /** What the action reads where it runs. */
    readonly context: ActionContext;
    /** Read some objects' rows of a type, leaving out those gone or that the person cannot read. */
    readonly read: (
        type: ObjectType,
        references: readonly ObjectReference[],
    ) => Promise<readonly JsonObject[]>;
}

/** The query of an action's targets: those it applies to that the person may call it on. */
export interface ActionQuery<Object extends ObjectType> {
    /** The condition the targets meet. */
    readonly where: ConditionOf<Object, Object>;
    /** The permission the person has on each target, the method's. */
    readonly permission: PermissionOf<Object>;
}

/** The method call an action makes: a template over its inputs, the same on every side. */
export class ActionCall {
    /** The object type the action calls. */
    readonly object: ObjectType;
    /** The method the action calls. */
    readonly method: string;
    /** The inputs the template reads. */
    readonly inputs: readonly ActionInput[];
    /** The method's input over the inputs. */
    readonly template: Template;
    /** The name of the input with the objects it calls, absent for a call on its scope. */
    readonly target: string | undefined;

    /** Create a call, refusing inputs the template cannot read. */
    constructor(
        name: string,
        definition: Pick<ActionCall, "object" | "method" | "inputs" | "template" | "target">,
    ) {
        // take the method, its inputs and its template
        this.object = definition.object;
        this.method = definition.method;
        this.inputs = definition.inputs;
        this.template = definition.template;
        this.target = definition.target;

        // refuse a reserved name, a variable of no input, and several objects for one value
        const names = this.inputs.map((each) => each.name);
        const reserved = names.find((each) => Template.isReserved(each));
        const unknown = this.template.inputs.find((each) => !names.includes(each));
        const several = this.inputs.find(
            (each) => each.objects?.source === "selected" && each.name !== this.target,
        );
        if (reserved !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `action '${name}' is invalid: its input '${reserved}' has a name calls read for themselves`,
            );
        } else if (unknown !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `action '${name}' is invalid: its call reads '${unknown}', which is no input of it`,
            );
        } else if (several !== undefined) {
            throw PackageError.of(
                "DEFINITION_INVALID",
                `action '${name}' is invalid: its input '${several.name}' takes several objects, but the call acts on others`,
            );
        }
    }

    /** Build an action's call from its inputs and the call its definition prepares. */
    static prepared(name: string, inputs: ActionInputs, prepared: ActionTemplate): ActionCall {
        return new ActionCall(name, {
            object: prepared.object,
            method: prepared.method,
            inputs: Object.entries(inputs).map(([input, definition]) =>
                ActionInput.of(input, definition),
            ),
            template: Template.of(prepared.input),
            target: prepared.target,
        });
    }

    /** Build the call of a method's own action: its objects, and every input it takes. */
    static of(object: ObjectType, method: string): ActionCall {
        // take each input the method takes, and the selected objects where it acts on them
        const inputs = object.inputs(method);
        const target = new ActionInput({
            name: TARGET_INPUT,
            label: undefined,
            presence: "required",
            value: { kind: "object", type: object, source: "selected", where: undefined },
        });
        const isTargeted = object.method(method).target !== "scope";
        const input = Object.fromEntries(inputs.map(({ name }) => [name, new Variable(name)]));

        return new ActionCall(method, {
            object,
            method,
            inputs: isTargeted ? [target, ...inputs] : inputs,
            template: Template.of(input),
            target: isTargeted ? TARGET_INPUT : undefined,
        });
    }

    /** Whether the call removes its target for good, which people confirm first. */
    get isDestructive(): boolean {
        return this.object.isDestructive(this.method);
    }

    /** Whether the call takes everything its method takes, presetting nothing. */
    get isWhole(): boolean {
        const taken = this.object.inputs(this.method);

        return taken.every(({ name }) => this.template.inputAt(name) !== undefined);
    }

    /** Report whether the call applies to a row at a moment, by its target's condition. */
    applies(row: JsonObject, moment: Moment): boolean {
        const where = this.#where(this.target);

        return where === undefined || Condition.matches(where, this.object.table, row, moment);
    }

    /** Read the value a target has for an input, absent for one that sets no field of it. */
    chosen(input: ActionInput, scope: string, row: JsonObject): JsonValue | undefined {
        // read the field the input sets
        const field = this.target === undefined ? undefined : this.template.fieldOf(input.name);
        const { value } = input;
        if (field === undefined) {
            return undefined;
        } else if (value.kind !== "object") {
            return row[field];
        }

        // read an object by its columns, else by its identifier in the target's scope
        const relation = Object.keys(this.object.fields).find(
            (name) =>
                (name === field || Field.columnsOf(name).id === field) &&
                Field.columnsOf(name).packageId in row,
        );
        const id = row[relation === undefined ? field : Field.columnsOf(relation).id];
        if (typeof id !== "string") {
            return id;
        }

        return relation === undefined
            ? value.type.reference(scope, id)
            : ObjectReference.at(relation, row);
    }

    /** Query the targets the call applies to: every one, or those among some objects. */
    query(ids?: readonly string[]): ActionQuery<ObjectType> {
        // match the condition, and the given objects alone when some are given
        const target = ids === undefined ? {} : { id: { in: [...ids] } };
        const condition = this.#where(this.target);
        const where = condition === undefined ? target : { AND: [target, condition] };

        // require the method's permission on each target
        const { permission } = this.object.method(this.method);
        if (permission === undefined || !this.object.isPermission(permission)) {
            throw new TypeError(`'${this.object.name}.${this.method}' declares no permission`);
        }

        return { where, permission };
    }

    /** Resolve the inputs from what is given and the places around, the nearest first. */
    resolve({ places }: Pick<ActionContext, "places">, given: ActionValues): ActionResolution {
        // give what a created object starts with to the inputs setting it, and the rest to the object
        const starting = Object.entries(
            this.target === undefined ? creating(this.object).read(places) : {},
        );
        const initial = starting.filter(([field]) => !(field in this.template.value));
        const started = starting.flatMap(([field, value]): [string, JsonValue | undefined][] => {
            const input = this.template.inputAt(field);

            return input === undefined ? [] : [[input, value]];
        });

        // take what places show over that, and what is given over both
        const shown = this.inputs.flatMap((each): [string, JsonValue][] => {
            const value = each.shown(places);

            return value === undefined ? [] : [[each.name, value]];
        });
        const values = { ...Object.fromEntries([...started, ...shown]), ...given };

        return {
            values,
            initial: Object.fromEntries(initial),
            asked: ActionInput.asked(this.inputs, values),
        };
    }

    /** Write the calls of resolved inputs, refusing objects the action does not apply to. */
    async invocations(
        resolved: ActionArguments,
        runner: ActionRunner,
    ): Promise<ActionInvocation[]> {
        // refuse a missing input, and one whose value it does not take
        const { values, initial = {} } = resolved;
        const { object, method } = this;
        for (const input of this.inputs) {
            const value = values[input.name];
            if (value === undefined && input.presence === "required") {
                throw ObjectError.of(
                    "CALL_BLOCKED",
                    `cannot call '${object.name}.${method}': input '${input.name}' is missing`,
                );
            } else if (value !== undefined && !input.takes(value)) {
                throw refuseInput(values, [
                    {
                        path: [input.name],
                        message: `it takes ${input.value.kind === "object" ? input.value.type.name : input.value.kind}`,
                    },
                ]);
            }
        }

        // read the rows the template and the conditions need
        const { context } = runner;
        const rows = new Map<string, JsonObject>();
        for (const input of this.inputs) {
            await this.#load(input, values, rows, runner);
        }

        // call the method once for a scope, else once for each target
        const { target } = this;
        if (target === undefined) {
            const input = this.#spread(this.template.bind({ values, rows, context }));

            return [{ scope: undefined, input: { ...initial, ...input } }];
        }

        return ActionCall.#references(values[target]).map((reference) => {
            const given = { values: { ...values, [target]: reference }, rows, context };

            return {
                scope: reference.scope,
                input: {
                    ...this.#spread(this.template.bind(given)),
                    [TARGET_FIELD]: reference.id,
                },
            };
        });
    }

    /** Describe the call by its type, method, inputs and template. */
    describe(): ActionCallDescription {
        return {
            packageId: this.object.package.id,
            type: this.object.name,
            method: this.method,
            input: this.template.value,
            ...(this.target === undefined ? {} : { target: this.target }),
            inputs: this.inputs.map((each) => each.describe()),
        };
    }

    /** Read a call from its description, finding the types of its objects. */
    static from(
        name: string,
        object: ObjectType,
        description: ActionCallDescription,
        typeOf: (reference: ObjectTypeReference) => ObjectType,
    ): ActionCall {
        const { method, input, target, inputs } = description;

        return new ActionCall(name, {
            object,
            method,
            inputs: inputs.map((each) => ActionInput.from(each, typeOf)),
            template: new Template(input),
            target,
        });
    }

    /** Read the rows of an input's objects, refusing those its condition leaves out. */
    async #load(
        input: ActionInput,
        values: ActionValues,
        rows: Map<string, JsonObject>,
        runner: ActionRunner,
    ): Promise<void> {
        // leave an input without objects, or whose rows nothing reads, alone
        const { objects, name } = input;
        const where = this.#where(name);
        const isRead = where !== undefined || this.template.reads.includes(name);
        if (objects === undefined || !isRead || values[name] === undefined) {
            return;
        }

        // read the rows at once, and refuse one that is gone or does not meet the condition
        const references = ActionCall.#references(values[name]);
        const read = await runner.read(objects.type, references);
        for (const reference of references) {
            const row = read.find((each) => each["id"] === reference.id);
            if (
                row === undefined ||
                (where !== undefined &&
                    !Condition.matches(where, objects.type.table, row, runner.context.moment))
            ) {
                throw ObjectError.of(
                    "CALL_BLOCKED",
                    `cannot call '${this.object.name}.${this.method}': it does not apply to ${reference.type} '${reference.id}'`,
                );
            }
            rows.set(ObjectReference.key(reference), row);
        }
    }

    /** Write each reference as its field stores it: in its columns, or by its identifier. */
    #spread(input: JsonObject): JsonObject {
        return Object.fromEntries(
            Object.entries(input).flatMap(([name, value]): [string, JsonValue | undefined][] => {
                // leave a value that is no reference as it is
                const reference = ObjectReference.safeParse(value).data;
                const field = this.object.fields[Field.columnsOf(name).id];
                if (reference === undefined) {
                    return [[name, value]];
                } else if (field?.reference === "object" && field.relation === name) {
                    return Object.entries(ObjectReference.columns(name, reference));
                }
                const isIdentifier = this.object.fields[name]?.reference === "identifier";

                return [[name, isIdentifier ? reference.id : value]];
            }),
        );
    }

    /** Read the condition an input's objects meet, none for another input. */
    #where(name: string | undefined) {
        return this.inputs.find((each) => each.name === name)?.objects?.where;
    }

    /** List the references of objects given: one, or several. */
    static #references(value: JsonValue | undefined): ObjectReference[] {
        return schema.array(ObjectReference).parse(Array.isArray(value) ? value : [value]);
    }
}

/** The method call an action makes, as its declaration describes it. */
export const ActionCallDescription = schema.object({
    /** The package declaring the object type. */
    packageId: PackageId,
    /** The object type the action calls. */
    type: schema.string().min(1),
    /** The method the action calls. */
    method: schema.string().min(1),
    /** The method's input, each variable by its name. */
    input: schema.jsonObject(),
    /** The name of the input with the objects it calls, absent for a call on its scope. */
    target: schema.string().min(1).exactOptional(),
    /** The inputs the call reads. */
    inputs: schema.array(ActionInputDescription),
});
/** The method call an action makes, as its declaration describes it. */
export type ActionCallDescription = schema.Infer<typeof ActionCallDescription>;
