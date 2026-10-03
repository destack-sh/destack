import { aligned, found } from "@destack/schema";
import { S3Error } from "./error.ts";

/** The namespace of S3 response documents. */
const NAMESPACE = "http://s3.amazonaws.com/doc/2006-03-01/";
/** The XML declaration S3 documents start with. */
const DECLARATION = '<?xml version="1.0" encoding="UTF-8"?>';
/** The characters XML text escapes, with their references. */
const ESCAPES = new Map([
    ["&", "&amp;"],
    ["<", "&lt;"],
    [">", "&gt;"],
    ['"', "&quot;"],
    ["'", "&apos;"],
]);
/** The predefined XML entities and their characters. */
const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };
/**
 * The XML tokens request documents use.
 *
 * The tokens are declarations, comments, CDATA, markup declarations, end tags, start tags and text.
 * Attributes are matched and ignored, since S3 request documents carry only the namespace.
 */
const TOKEN =
    /<\?[\s\S]*?\?>|<!--[\s\S]*?-->|<!\[CDATA\[([\s\S]*?)\]\]>|(<![^>]*>)|<\/([^\s>]+)\s*>|<([^\s/>]+)(?:\s+[^\s=/>]+\s*=\s*(?:"[^"]*"|'[^']*'))*\s*(\/?)>|([^<]+)/uy;

/** A value an XML element has: text, nested elements, or nothing. */
export type XmlValue = string | number | boolean | Date | XmlElement | undefined;

/** An element with its children, written in order. */
export interface XmlElement {
    /** The child elements by name, with arrays repeating the name. */
    readonly [name: string]: XmlValue | readonly XmlValue[];
}

/** A parsed element of a request document. */
export class XmlNode {
    /** The local element name, without a namespace prefix. */
    readonly name: string;
    /** The child elements. */
    readonly children: XmlNode[] = [];
    /** The concatenated text content. */
    text = "";

    /** Create an element without content. */
    constructor(name: string) {
        this.name = name;
    }

    /** Parse a request document, refusing malformed XML and markup declarations. */
    static parse(document: string): XmlNode {
        // collect elements on a stack as their tags open and close
        const root = new XmlNode("");
        const open = [root];
        TOKEN.lastIndex = 0;
        while (TOKEN.lastIndex < document.length) {
            const match = TOKEN.exec(document);
            const current = aligned(open, open.length - 1);
            // refuse text the tokenizer cannot read and document type declarations
            if (match === null || match[2] !== undefined) {
                throw new S3Error(
                    "MalformedXML",
                    "the XML document is not well-formed or uses unsupported markup",
                );
            }
            // close the current element by its name
            else if (match[3] !== undefined) {
                if (open.length === 1 || localName(match[3]) !== current.name) {
                    throw new S3Error(
                        "MalformedXML",
                        "the XML document closes an element it did not open",
                    );
                }
                open.pop();
            }
            // open an element, closing it at once when empty
            else if (match[4] !== undefined) {
                const element = new XmlNode(localName(match[4]));
                current.children.push(element);
                if (match[5] !== "/") {
                    open.push(element);
                }
            }
            // collect character data
            else if (match[1] !== undefined) {
                current.text += match[1];
            }
            // collect text
            else if (match[6] !== undefined) {
                current.text += decodeText(match[6]);
            }
        }

        // require exactly one closed root element
        if (open.length !== 1 || root.children.length !== 1) {
            throw new S3Error(
                "MalformedXML",
                "the XML document must have exactly one root element",
            );
        }

        return aligned(root.children, 0);
    }

    /** Select the child elements with a name. */
    all(name: string): XmlNode[] {
        return this.children.filter((child) => child.name === name);
    }

    /** Read the text of the only child with a name, or undefined when absent. */
    value(name: string): string | undefined {
        const [child, ...rest] = this.all(name);
        if (rest.length > 0) {
            throw new S3Error("MalformedXML", `the XML element ${this.name} repeats ${name}`);
        }

        return child?.text;
    }
}

/** Write an S3 response document with one root element. */
export function writeXml(name: string, root: XmlElement): string {
    return `${DECLARATION}<${name} xmlns="${NAMESPACE}">${writeChildren(root)}</${name}>`;
}

/** Write an S3 error document, which S3 writes without a namespace. */
export function writeErrorXml(root: XmlElement): string {
    return `${DECLARATION}<Error>${writeChildren(root)}</Error>`;
}

/** Write the children of an element in order, skipping undefined values. */
function writeChildren(element: XmlElement): string {
    let output = "";
    for (const [name, value] of Object.entries(element)) {
        for (const item of isList(value) ? value : [value]) {
            output += item === undefined ? "" : `<${name}>${writeValue(item)}</${name}>`;
        }
    }

    return output;
}

/** Report whether an element's value repeats its name as a list. */
function isList(value: XmlValue | readonly XmlValue[]): value is readonly XmlValue[] {
    return Array.isArray(value);
}

/** Write one element value. */
function writeValue(value: Exclude<XmlValue, undefined>): string {
    // write nested elements
    if (typeof value === "object" && !(value instanceof Date)) {
        return writeChildren(value);
    }
    // write times as ISO 8601
    else if (value instanceof Date) {
        return value.toISOString();
    }
    // write escaped text
    else {
        return String(value).replace(/[&<>"']/gu, (character) => found(ESCAPES, character));
    }
}

/** Decode the entity and character references of XML text. */
function decodeText(text: string): string {
    return text.replace(
        /&(#x[0-9a-fA-F]+|#[0-9]+|[a-z]+);|&/gu,
        (reference, name: string | undefined) => {
            // refuse a bare ampersand and unknown entities
            if (name === undefined) {
                throw new S3Error("MalformedXML", "the XML text has a bare ampersand");
            }
            // decode character references
            else if (name.startsWith("#")) {
                const codepoint =
                    name[1] === "x" ? Number.parseInt(name.slice(2), 16) : Number(name.slice(1));
                if (codepoint > 0x10ffff) {
                    throw new S3Error(
                        "MalformedXML",
                        `the XML text references an invalid character ${reference}`,
                    );
                }

                return String.fromCodePoint(codepoint);
            }
            // decode the predefined entities
            else {
                const character = ENTITIES[name];
                if (character === undefined) {
                    throw new S3Error(
                        "MalformedXML",
                        `the XML text references an unknown entity ${reference}`,
                    );
                }

                return character;
            }
        },
    );
}

/** Strip a namespace prefix from an element name. */
function localName(name: string): string {
    return name.slice(name.indexOf(":") + 1);
}
