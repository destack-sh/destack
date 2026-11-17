import { type JsonValue, schema } from "@destack/schema";
import type { TextNode } from "./element.ts";
import type { TextEdit } from "./edit.ts";
import type { TextMark } from "./mark.ts";

/** How a mark's range grows when text is inserted at its edges. */
export type TextExpand = "before" | "after" | "both" | "none";

/** A type of mark a text schema allows, such as bold or a link. */
export interface TextMarkType {
    /** The type's name. */
    readonly name: string;
    /** How a mark's range grows at its edges. */
    readonly expand: TextExpand;
    /** Whether marks of the type overlap, such as several people's highlights. */
    readonly isMultiple: boolean;
    /** The mark types this one removes where it applies, such as code removing bold. */
    readonly excludes: readonly string[];
    /** The schema of a mark's value, true for a plain style. */
    readonly value: schema.Schema<JsonValue>;
}

/** A type of node a text schema allows: a block starting a line, or an inline node. */
export interface TextNodeType {
    /** The type's name. */
    readonly name: string;
    /** Whether a node of the type starts a block, such as a heading. */
    readonly isBlock: boolean;
    /** The schema of a node's value. */
    readonly value: schema.Schema<JsonValue>;
    /** Whether the store indexes a node's value as a reference, such as a mention's subject. */
    readonly isReferencing: boolean;
}

/** The value of a plain style: present or absent. */
const STYLE = schema.literal(true);

/** The marks and nodes one text field allows. */
export class TextSchema {
    /** The allowed mark types by name. */
    readonly marks: ReadonlyMap<string, TextMarkType>;
    /** The allowed node types by name. */
    readonly nodes: ReadonlyMap<string, TextNodeType>;

    /** Allow some mark and node types. */
    constructor(marks: readonly TextMarkType[], nodes: readonly TextNodeType[]) {
        this.marks = new Map(marks.map((type) => [type.name, type]));
        this.nodes = new Map(nodes.map((type) => [type.name, type]));
    }

    /** Compose a schema of mark and node types, refusing two types of one name. */
    static of(...types: readonly (TextMarkType | TextNodeType)[]): TextSchema {
        // refuse a name given twice
        const names = types.map((type) => type.name);
        const repeated = names.find((name, index) => names.indexOf(name) !== index);
        if (repeated !== undefined) {
            throw new TypeError(`text type '${repeated}' is declared twice`);
        }

        // split the types into marks and nodes
        return new TextSchema(
            types.filter((type): type is TextMarkType => "expand" in type),
            types.filter((type): type is TextNodeType => "isBlock" in type),
        );
    }

    /** Declare a mark type, by default a style that grows after its end. */
    static mark(
        name: string,
        options: {
            readonly expand?: TextExpand;
            readonly isMultiple?: boolean;
            readonly excludes?: readonly string[];
            readonly value?: schema.Schema<JsonValue>;
        } = {},
    ): TextMarkType {
        return {
            name,
            expand: options.expand ?? "after",
            isMultiple: options.isMultiple ?? false,
            excludes: options.excludes ?? [],
            value: options.value ?? STYLE,
        };
    }

    /** Declare a node type: inline by default, with no value beyond its type. */
    static node(
        name: string,
        options: {
            readonly isBlock?: boolean;
            readonly value?: schema.Schema<JsonValue>;
            readonly isReferencing?: boolean;
        } = {},
    ): TextNodeType {
        return {
            name,
            isBlock: options.isBlock ?? false,
            value: options.value ?? schema.null(),
            isReferencing: options.isReferencing ?? false,
        };
    }

    /** Require the nodes and the mark of an edit the schema allows. */
    require(edit: TextEdit): void {
        // check an inserted, restored or set node, and an added mark
        if ("insert" in edit && typeof edit.insert !== "string") {
            this.#requireNode(edit.insert);
        } else if ("restore" in edit) {
            for (const piece of edit.restore) {
                if ("node" in piece) {
                    this.#requireNode(piece.node);
                }
            }
        } else if ("set" in edit) {
            this.#requireNode(edit.node);
        } else if ("mark" in edit) {
            this.#requireMark(edit.mark);
        }
    }

    /** Require a node the schema allows, with a valid value. */
    #requireNode(node: TextNode): void {
        // refuse an unknown type, then a value its schema refuses
        const type = this.nodes.get(node.type);
        if (type === undefined) {
            throw new RangeError(`text node ${node.type} is invalid: the field does not allow it`);
        } else if (!type.value.safeParse(node.value).success) {
            throw new RangeError(
                `text node ${node.type} is invalid: its value does not match its schema`,
            );
        }
    }

    /** Require a mark the schema allows, with a valid value or null removing a single type. */
    #requireMark(mark: Pick<TextMark, "type" | "value">): void {
        // refuse unknown types and invalid values
        const type = this.marks.get(mark.type);
        if (type === undefined) {
            throw new RangeError(`text mark ${mark.type} is invalid: the field does not allow it`);
        } else if (mark.value === null && type.isMultiple) {
            throw new RangeError(
                `text mark ${mark.type} is invalid: overlapping marks are removed by identifier`,
            );
        } else if (mark.value !== null && !type.value.safeParse(mark.value).success) {
            throw new RangeError(
                `text mark ${mark.type} is invalid: its value does not match its schema`,
            );
        }
    }

    /** Read a mark type the schema allows, refusing an unknown one. */
    markType(type: string): TextMarkType {
        const declared = this.marks.get(type);
        if (declared === undefined) {
            throw new RangeError(`text mark ${type} is invalid: the field does not allow it`);
        }

        return declared;
    }
}

/** The plain text schema: characters alone, no marks and no nodes. */
export const PLAIN_TEXT = new TextSchema([], []);
