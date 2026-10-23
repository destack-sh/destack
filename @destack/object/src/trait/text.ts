import { present, schema } from "@destack/schema";
import { ObjectError } from "../error/error.ts";

import { Step } from "../method/step.ts";
import type { TextField } from "../field/field.ts";
import { defineMethod, Method } from "../method/method.ts";
import type { Procedure, ReplayFields, TargetFields } from "../method/procedure.ts";
import type { ObjectType, TextFieldName } from "../object/object.ts";
import { Element, SequenceEdit } from "../sequence/index.ts";
import { permission } from "@destack/access";
import { Chunk, Containment, Undo, TEXT_READ } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";
import type { Trait } from "./trait.ts";
import { Execution } from "../method/execution.ts";

import type {} from "@destack/package/import-meta";

/** The text fields of an object and the permission editing them. */
export interface TextDefinition {
    /** The text fields, by name. */
    readonly fields: readonly [string, ...string[]];
    /** The permission edits need. */
    readonly permission: string | undefined;
    /** The permission reading the text. */
    readonly reading: string | undefined;
}

/** The method editing an object's text fields. */
export type TextMethodMap<Fields> = {
    [Property in keyof Fields]: Fields[Property] extends TextField ? Property : never;
}[keyof Fields] extends never
    ? {}
    : {
          readonly edit: Method<{
              kind: "edit";
              permission: string;
              output: typeof Undo;
              mutates: true;
          }>;
          readonly contains: Method<{
              kind: "contains";
              permission: string;
              output: typeof Containment;
              mutates: false;
          }>;
      };

/** The procedures of the edit and contains methods. */
export type TextProcedures<Object extends ObjectType> = {
    edit: Procedure<
        schema.Object<
            TargetFields<Object> &
                ReplayFields<Object> & {
                    field: schema.Schema<TextFieldName<Object["fields"]>>;
                    edits: schema.Schema<SequenceEdit[]>;
                }
        >,
        typeof Undo
    >;
    contains: Procedure<
        schema.Object<
            TargetFields<Object> & {
                field: schema.Schema<TextFieldName<Object["fields"]>>;
                elements: schema.Schema<Element[]>;
            }
        >,
        typeof Containment
    >;
};

/** Text fields stored in chunks and changed only by edits. */
export const text: Trait<TextDefinition> & {
    /** Edit one text field of an object. */
    readonly edit: (
        required: string,
        fields: readonly [string, ...string[]],
    ) => Method<{ kind: "edit"; permission: string; output: typeof Undo; mutates: true }>;
    /** Decide whether one text field of an object has some elements, as its text readers may. */
    readonly contains: (fields: readonly [string, ...string[]]) => Method<{
        kind: "contains";
        permission: string;
        output: typeof Containment;
        mutates: false;
    }>;
} = {
    options: (definition) => {
        // collect the text fields and the update and reading permissions
        const [first, ...others] = Object.entries(definition.fields ?? {})
            .filter(([, declared]) => declared.type === "text")
            .map(([name]) => name);
        const methods = Object.values(definition.methods ?? {});
        const readPermission = (kind: string) =>
            methods.find((declared) => declared.kind === kind)?.permission ?? undefined;

        return first === undefined
            ? undefined
            : {
                  fields: [first, ...others],
                  permission: readPermission("update"),
                  reading: readPermission("list") ?? readPermission("get"),
              };
    },
    columns: () => ({}),
    constraints: () => [],
    // delete chunks with their owner
    table: (_options, object) => ({
        dependents: [
            {
                from: () => chunk,
                key: "parentId",
                where: { parentPackageId: object.packageId, parentType: object.name },
                onDelete: "cascade",
            },
        ],
    }),
    policy: (options, object, permissions) => {
        // derive the text permission from the reading one
        if (permissions.includes(TEXT_READ)) {
            throw new TypeError(
                `object ${object.name} cannot declare permission ${TEXT_READ}: its text fields derive it`,
            );
        }

        return options.reading === undefined
            ? {}
            : { permissions: { [TEXT_READ]: permission(options.reading) } };
    },
    methods: (options): Record<string, Method> =>
        options.permission === undefined
            ? {}
            : {
                  edit: text.edit(options.permission, options.fields),
                  contains: text.contains(options.fields),
              },
    validate: (options, object) => {
        // require durable objects with an update permission
        if (object.storage === "ephemeral") {
            throw new TypeError(`ephemeral object ${object.name} has no text`);
        } else if (options.permission === undefined) {
            throw new TypeError(`object ${object.name} has text but no update method to edit it`);
        } else if (options.reading === undefined) {
            throw new TypeError(
                `object ${object.name} has text but no get or list method to read it`,
            );
        }

        // refuse methods writing text fields
        for (const [name, declared] of Object.entries(object.methods)) {
            const written = declared.fields?.find((field) => options.fields.includes(field));
            if (written !== undefined) {
                throw new TypeError(
                    `method ${name} of ${object.name} cannot write text field ${written}: only edit changes it`,
                );
            }
        }
    },
    edit(
        required: string,
        fields: readonly [string, ...string[]],
    ): Method<{ kind: "edit"; permission: string; output: typeof Undo; mutates: true }> {
        return defineMethod<{
            kind: "edit";
            permission: string;
            output: typeof Undo;
            mutates: true;
        }>({
            kind: "edit",
            permission: required,
            mutates: true,
            target: "object",
            result: "value",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/edit" },
                input: schemas.target.extend({
                    ...schemas.replay,
                    field: schema.enum(fields),
                    edits: schema.array(SequenceEdit).min(1),
                }),
                output: Undo,
            }),
            handler: (context, execution) => Chunk.edit(context, execution),
            inverse: (step) => {
                // apply the recorded inverse edits
                if (step.result === undefined) {
                    return undefined;
                }
                const result = Undo.parse(step.result);

                return result.inverse.length === 0
                    ? []
                    : [
                          Step.record(step, step.name, {
                              ...Step.target(step),
                              field: schema.string().parse(step.input["field"]),
                              edits: result.inverse,
                          }),
                      ];
            },
            async execute(context, execution) {
                // require the field's write permission when it guards writes
                const field = schema.string().parse(context.input["field"]);
                const guard = present(
                    context.object.fields[field],
                    `the text field ${field} of ${context.object.name}`,
                ).access?.write;
                if (!context.isPredicted && guard !== undefined) {
                    const decision = await context.check(
                        context.object.permission(guard),
                        context.reference(),
                    );
                    if (!decision.isAllowed) {
                        throw ObjectError.of(
                            "FIELD_NOT_WRITABLE",
                            `field ${field} is not writable`,
                        );
                    }
                }

                return Method.handle(this, context, execution);
            },
        });
    },
    contains(fields: readonly [string, ...string[]]): Method<{
        kind: "contains";
        permission: string;
        output: typeof Containment;
        mutates: false;
    }> {
        return defineMethod<{
            kind: "contains";
            permission: string;
            output: typeof Containment;
            mutates: false;
        }>({
            kind: "contains",
            permission: TEXT_READ,
            mutates: false,
            target: "object",
            result: "value",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/contains" },
                input: schemas.target.extend({
                    field: schema.enum(fields),
                    elements: schema.array(Element).min(1),
                }),
                output: Containment,
            }),
            handler: async (context, execution) => {
                // look the elements up among the field's runs in the call's view
                const field = schema.string().parse(context.input["field"]);
                const elements = schema.array(Element).parse(context.input["elements"]);
                const isContained = await Chunk.contains(
                    Execution.snapshot(execution),
                    context.reference(),
                    field,
                    elements,
                );

                return { isContained };
            },
        });
    },
};
