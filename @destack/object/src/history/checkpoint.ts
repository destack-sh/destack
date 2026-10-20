import { through } from "@destack/access";
import { LogPosition } from "@destack/db";
import { schema } from "@destack/schema";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/record.ts";
import { defineObject } from "../object/object.ts";
import type { ObjectTable } from "../object/table.ts";

/** The checkpoints of a scope: named points in an object's history. */
export const checkpoint = defineObject(
    {
        name: "checkpoint",
        plural: "checkpoints",
        nested: { in: "any", receive: "checkpoint" },
        fields: {
            /** The log position the checkpoint marks. */
            position: field.json(LogPosition),
            /** The checkpoint's name. */
            title: field.string(schema.string().min(1).max(200)),
            /** What the checkpoint marks. */
            description: field.string(schema.string().max(2000)).optional(),
        },
        permissions: {
            read: through("parent", "read"),
            checkpoint: through("parent", "checkpoint"),
        },
    },
    <Definition extends ObjectTable>(method: MethodBuilder<Definition>) => ({
        create: method
            .create("checkpoint", { fields: ["title", "description"], isPredicted: false })
            .handle(async (context, next) =>
                next(context.with({ input: { ...context.input, position: context.base } })),
            ),
        update: method.update("checkpoint", { fields: ["title", "description"] }),
        delete: method.delete("checkpoint"),
    }),
);
