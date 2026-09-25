import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import { escapeHtml } from "../src/content/html.ts";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("rs", rust);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("ts", typescript);
hljs.registerLanguage("tsx", typescript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("svg", xml);

/// Highlight one source string.
export function highlightCode(source: string, language: string): string {
    const normalized = normalizeLanguage(language);

    // keep plain text unhighlighted
    if (normalized === "text") {
        return escapeHtml(source);
    }
    // use the registered grammar of a known language
    else if (normalized !== "" && hljs.getLanguage(normalized) != undefined) {
        return hljs.highlight(source, { language: normalized }).value;
    }
    // detect the language of unlabeled and unregistered fences
    else {
        return hljs.highlightAuto(source).value;
    }
}

/// Normalize one Markdown language identifier.
function normalizeLanguage(language: string) {
    return language.trim().split(/[:\s]+/)[0].toLowerCase().replace(/^\./, "");
}
