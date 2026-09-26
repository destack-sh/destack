import type { Rule } from "@oxlint/plugins";
import { isDeclaredName, splitWords } from "./word.ts";

/** Abbreviations and the words they shorten. */
const ABBREVIATIONS: Readonly<Record<string, string>> = {
    arg: "argument",
    args: "arguments",
    btn: "button",
    buf: "buffer",
    cb: "callback",
    cfg: "configuration",
    conf: "configuration",
    config: "configuration",
    ctx: "context",
    cur: "current",
    def: "definition",
    del: "delete",
    desc: "description",
    dest: "destination",
    dir: "directory",
    dirs: "directories",
    doc: "document",
    docs: "documents",
    el: "element",
    env: "environment",
    err: "error",
    evt: "event",
    ext: "extension",
    fn: "function",
    idx: "index",
    impl: "implementation",
    init: "initialize",
    len: "length",
    lib: "library",
    msg: "message",
    num: "number",
    obj: "object",
    opt: "option",
    opts: "options",
    param: "parameter",
    params: "parameters",
    pkg: "package",
    prev: "previous",
    prop: "property",
    props: "properties",
    ref: "reference",
    refs: "references",
    req: "request",
    res: "response",
    ret: "result",
    src: "source",
    str: "string",
    temp: "temporary",
    tmp: "temporary",
    util: "utility",
    val: "value",
    var: "variable",
};

/** Reject abbreviated words in declared names. */
export const preventAbbreviations: Rule = {
    meta: {
        type: "suggestion",
        schema: [],
        docs: { description: "write out words in names" },
        messages: { word: "[WN14] write out '{{word}}' as '{{replacement}}'" },
    },
    create(context) {
        return {
            Identifier(node) {
                // check names where they are declared
                const word = isDeclaredName(node)
                    ? splitWords(node.name).find((entry) => Object.hasOwn(ABBREVIATIONS, entry))
                    : undefined;
                if (word) {
                    context.report({
                        node,
                        messageId: "word",
                        data: { word, replacement: ABBREVIATIONS[word] },
                    });
                }
            },
        };
    },
};
