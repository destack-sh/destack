import { present, schema } from "@destack/schema";
import { ObjectError } from "../error/error.ts";

import { Step } from "../method/step.ts";
import type { TextField } from "../field/field.ts";
import { defineMethod, Method } from "../method/method.ts";
import type { Procedure, ReplayFields, TargetFields } from "../method/procedure.ts";
import type { ObjectType, TextFieldName } from "../object/object.ts";
import { TextElement, TextEdit } from "../sequence/index.ts";
import { permission } from "@destack/access";
import { Chunk, Containment, Undo } from "../text/chunk.ts";
import { TEXT_READ, textChunkPolicy, textNodePolicy } from "../text/policy.ts";
import { textChunkTable, textNodeTable } from "../text/table.ts";
import type { TraitBehavior } from "./trait.ts";
import { Execution } from "../method/execution.ts";

import type {} from "@destack/package/import-meta";
import { t } from "@destack/locale";

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
                    edits: schema.Schema<TextEdit[]>;
                }
        >,
        typeof Undo
    >;
    contains: Procedure<
        schema.Object<
            TargetFields<Object> & {
                field: schema.Schema<TextFieldName<Object["fields"]>>;
                elements: schema.Schema<TextElement[]>;
            }
        >,
        typeof Containment
    >;
};

/** Text fields stored in chunks, written at creation and by edits after. */
export const text: TraitBehavior<TextDefinition> = {
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
    // delete chunks and nodes with their owner
    table: (_options, object) => ({
        dependents: [textChunkTable, textNodeTable].map((table) => ({
            from: () => table,
            key: "parentId",
            where: { parentPackageId: object.packageId, parentType: object.name },
            onDelete: "cascade" as const,
        })),
    }),
    policy: (options, object, permissions) => {
        // derive the text permission from the reading one
        if (permissions.includes(TEXT_READ)) {
            throw new TypeError(
                `object ${object.name} cannot declare permission '${TEXT_READ}': its text fields derive it`,
            );
        }

        // join the open parent relations of the text types, as an attachment joins its type's
        const contributes = [textChunkPolicy, textNodePolicy].map((policy) => ({
            policy,
            relation: "parent",
        }));

        return options.reading === undefined
            ? { contributes }
            : { permissions: { [TEXT_READ]: permission(options.reading) }, contributes };
    },
    methods: (options): Record<string, Method> =>
        options.permission === undefined
            ? {}
            : {
                  edit: Text.edit(options.permission, options.fields),
                  contains: Text.contains(options.fields),
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

        // refuse methods writing text fields, other than creations
        for (const [name, declared] of Object.entries(object.methods)) {
            const written = declared.fields?.find((field) => options.fields.includes(field));
            if (written !== undefined && declared.kind !== "create") {
                throw new TypeError(
                    `method '${name}' of ${object.name} cannot write text field ${written}: only creations and edit write it`,
                );
            }
        }
    },
};

/** The methods of text fields: editing them and checking what they contain. */
export const Text = {
    /** Edit one text field of an object. */
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
            title: t.context("action")`Edit`,
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
                    edits: schema.array(TextEdit).min(1),
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
            compose: (previous, next) => Chunk.compose(previous, next),
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
                            "FIELD_DENIED",
                            `field '${field}' is forbidden: it is not writable`,
                        );
                    }
                }

                return Method.handle(this, context, execution);
            },
        });
    },

    /** Decide whether one text field of an object has some elements, as its text readers may. */
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
            title: t.context("action")`Check containment`,
            kind: "contains",
            permission: TEXT_READ,
            mutates: false,
            target: "object",
            result: "value",
            procedure: (_name, schemas) => ({
                route: { method: "POST", path: "/{id}/contains" },
                input: schemas.target.extend({
                    field: schema.enum(fields),
                    elements: schema.array(TextElement).min(1),
                }),
                output: Containment,
            }),
            handler: async (context, execution) => {
                // look the elements up among the field's runs in the call's view
                const field = schema.string().parse(context.input["field"]);
                const elements = schema.array(TextElement).parse(context.input["elements"]);
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
