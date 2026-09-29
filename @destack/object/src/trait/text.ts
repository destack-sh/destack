import { schema } from "@destack/schema";
import type { Field, TextField } from "../field/field.ts";
import { Step } from "../method/step.ts";
import { defineMethod, type Method } from "../method/method.ts";
import type { Procedure, ReplayShape, TargetShape } from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import { SequenceEdit } from "../sequence/index.ts";
import { permission } from "@destack/access";
import { ServiceError } from "@destack/service/error";
import { Chunk, Edited, TEXT_READ } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";
import type { Trait } from "./trait.ts";

/** The text fields of an object and the permission editing them. */
export interface TextDefinition {
    /** The text fields, by name. */
    readonly fields: readonly string[];
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
    : { readonly edit: Method<"edit", string, never, typeof Edited, true> };

/** The procedure of the edit method. */
export type TextProcedures<Object extends ObjectType> = {
    edit: Procedure<
        schema.Object<
            TargetShape<Object> &
                ReplayShape<Object> & {
                    field: schema.Schema<Object["text"][number]>;
                    edits: schema.Schema<SequenceEdit[]>;
                }
        >,
        typeof Edited
    >;
};

/** Text fields held in chunks and changed only by edits. */
export const text: Trait<TextDefinition> = {
    options: (definition) => {
        // collect the text fields and the update and reading permissions
        const fields = Object.entries(definition.fields ?? {})
            .filter(([, declared]) => (declared as Field).type === "text")
            .map(([name]) => name);
        const methods = Object.values(
            (definition.methods ?? {}) as Readonly<Record<string, Method>>,
        );
        const permissionOf = (kind: string) =>
            methods.find((declared) => declared.kind === kind)?.permission ?? undefined;

        return fields.length === 0
            ? undefined
            : {
                  fields,
                  permission: permissionOf("update"),
                  reading: permissionOf("list") ?? permissionOf("get"),
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
                `object ${object.name} declares permission ${TEXT_READ}, which its text fields derive`,
            );
        }

        return options.reading === undefined
            ? {}
            : { permissions: { [TEXT_READ]: permission(options.reading) } };
    },
    methods: (options): Record<string, Method> =>
        options.permission === undefined ? {} : { edit: edit(options.permission, options.fields) },
    validate: (options, object) => {
        // require durable objects with an update permission
        if (object.storage === "ephemeral") {
            throw new TypeError(`ephemeral object ${object.name} holds no text`);
        } else if (options.permission === undefined) {
            throw new TypeError(`object ${object.name} holds text but no update method to edit it`);
        } else if (options.reading === undefined) {
            throw new TypeError(
                `object ${object.name} holds text but no get or list method reads it`,
            );
        }

        // refuse methods writing text fields
        for (const [name, declared] of Object.entries(
            object.methods as Readonly<Record<string, Method>>,
        )) {
            const written = declared.fields?.find((field) => options.fields.includes(field));
            if (written !== undefined) {
                throw new TypeError(
                    `method ${name} of ${object.name} writes text field ${written}, which only edit changes`,
                );
            }
        }
    },
};

/** Edit one text field of an object. */
function edit(
    permission: string,
    fields: readonly string[],
): Method<"edit", string, never, typeof Edited, true> {
    return defineMethod<Method<"edit", string, never, typeof Edited, true>>({
        kind: "edit",
        permission,
        mutates: true,
        target: true,
        result: "value",
        procedure: (_name, shapes) => ({
            route: { method: "POST", path: "/{id}/edit" },
            input: shapes.target.extend({
                ...shapes.replay,
                field: schema.enum(fields as [string, ...string[]]),
                edits: schema.array(SequenceEdit).min(1),
            }),
            output: Edited,
        }),
        effect: (call) => Chunk.edit(call),
        inverse: (step) => {
            // apply the recorded inverse edits
            const result = step.result as Edited | undefined;
            if (result === undefined) {
                return undefined;
            }

            return result.inverse.length === 0
                ? []
                : [
                      Step.record(step, step.name, {
                          ...Step.target(step, step.input.id),
                          field: step.input.field,
                          edits: result.inverse,
                      }),
                  ];
        },
        async execute(call) {
            // require the field's write permission when it guards writes
            const guard = call.object.fields[String(call.input.field)]!.access?.write;
            if (!call.isPredicted && guard !== undefined) {
                const decision = await call
                    .served()
                    .check(call.object.permission(guard), call.reference());
                if (!decision.isAllowed) {
                    throw new ServiceError("FORBIDDEN", {
                        message: `field ${String(call.input.field)} is not writable`,
                    });
                }
            }

            return this.effect(call);
        },
    });
}
