import { space } from "@destack/account/object";
import { principal, relation, through, union } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { nested, shareable } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** The kinds of a canvas's shapes. */
const SHAPE_KINDS = ["rectangle", "ellipse", "text"] as const;

/** A design file, whose members read and move its shapes. */
export const file = defineObject(
    {
        name: "file",
        plural: "files",
        title: t`${plural(argument("count"), { one: "File", other: "Files" })}`,
        scope: space,
        fields: {
            /** The principal who created the file. */
            owner: field.subject().caller(),
            /** The file's name. */
            name: field.string(schema.string().min(1).max(100)),
        },
        access: {
            relations: { member: { subjects: [principal.user] } },
            permissions: {
                read: union(relation("owner"), relation("member")),
                write: union(relation("owner"), relation("member")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({ create: method.create("manage", { fields: ["name"] }) }),
);

/** A shape on a file's canvas, at a position and size. */
export const shape = defineObject(
    {
        name: "shape",
        plural: "shapes",
        title: t`${plural(argument("count"), { one: "Shape", other: "Shapes" })}`,
        scope: space,
        fields: {
            /** What the shape draws. */
            kind: field.enum(SHAPE_KINDS).default("rectangle"),
            /** The left edge, in canvas units. */
            x: field.integer().default(0),
            /** The top edge, in canvas units. */
            y: field.integer().default(0),
            /** The width, in canvas units. */
            width: field.integer().default(100),
            /** The height, in canvas units. */
            height: field.integer().default(100),
        },
        access: {
            permissions: { read: through("parent", "read"), write: through("parent", "write") },
        },
        traits: [nested({ in: file, delete: "cascade", add: "write" })],
    },
    (method) => ({
        create: method.create("write", { fields: ["kind", "x", "y", "width", "height"] }),
        update: method.update("write", { fields: ["x", "y", "width", "height"] }),
    }),
);
