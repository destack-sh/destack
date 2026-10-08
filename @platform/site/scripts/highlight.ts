import type { ElementContent, RootContent } from "hast";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import { createLowlight } from "lowlight";

/** The highlighter with the languages the site's listings use. */
const lowlight = createLowlight({ bash, javascript, json, rust, typescript, xml });
lowlight.registerAlias({
    javascript: ["js"],
    rust: ["rs"],
    typescript: ["ts", "tsx"],
    xml: ["svg"],
});

/** Highlight one source string into the tree of its highlighted spans. */
export function highlightCode(source: string, language: string): ElementContent[] {
    const normalized = normalizeLanguage(language);

    // keep plain text unhighlighted
    if (normalized === "text") {
        return [{ type: "text", value: source }];
    }
    // use the registered grammar of a known language
    else if (normalized !== "" && lowlight.registered(normalized)) {
        return withoutDoctypes(lowlight.highlight(normalized, source).children);
    }
    // detect the language of unlabeled and unregistered fences
    else {
        return withoutDoctypes(lowlight.highlightAuto(source).children);
    }
}

/** Normalize one Markdown language identifier. */
function normalizeLanguage(language: string) {
    const [name = ""] = language.trim().split(/[:\s]+/u);

    return name.toLowerCase().replace(/^\./u, "");
}

/** Keep the highlighted spans and text of a highlighted tree, which holds no doctype. */
function withoutDoctypes(nodes: readonly RootContent[]): ElementContent[] {
    return nodes.filter((node): node is ElementContent => node.type !== "doctype");
}
