import { schema } from "@destack/schema";
import type { TextField } from "../field/field.ts";
import { type Method } from "../method/method.ts";
import type { Procedure, ReplayShape, TargetShape } from "../method/procedure.ts";
import type { ObjectType, TextFieldName } from "../object/object.ts";
import { SequenceEdit } from "../sequence/index.ts";
import { permission } from "@destack/access";
import { Undo, TEXT_READ } from "../text/chunk.ts";
import { chunk } from "../text/table.ts";
import type { Trait } from "./trait.ts";

import { edit } from "../method/text.ts";
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
      };

/** The procedure of the edit method. */
export type TextProcedures<Object extends ObjectType> = {
    edit: Procedure<
        schema.Object<
            TargetShape<Object> &
                ReplayShape<Object> & {
                    field: schema.Schema<TextFieldName<Object["fields"]>>;
                    edits: schema.Schema<SequenceEdit[]>;
                }
        >,
        typeof Undo
    >;
};

/** Text fields kept in chunks and changed only by edits. */
export const text: Trait<TextDefinition> = {
    options: (definition) => {
        // collect the text fields and the update and reading permissions
        const [first, ...others] = Object.entries(definition.fields ?? {})
            .filter(([, declared]) => declared.type === "text")
            .map(([name]) => name);
        const methods = Object.values(definition.methods ?? {});
        const permissionOf = (kind: string) =>
            methods.find((declared) => declared.kind === kind)?.permission ?? undefined;

        return first === undefined
            ? undefined
            : {
                  fields: [first, ...others],
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
            throw new TypeError(`ephemeral object ${object.name} has no text`);
        } else if (options.permission === undefined) {
            throw new TypeError(`object ${object.name} holds text but no update method to edit it`);
        } else if (options.reading === undefined) {
            throw new TypeError(
                `object ${object.name} has text but no get or list method reads it`,
            );
        }

        // refuse methods writing text fields
        for (const [name, declared] of Object.entries(object.methods)) {
            const written = declared.fields?.find((field) => options.fields.includes(field));
            if (written !== undefined) {
                throw new TypeError(
                    `method ${name} of ${object.name} writes text field ${written}, which only edit changes`,
                );
            }
        }
    },
};
