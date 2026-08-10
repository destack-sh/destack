import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import json from "highlight.js/lib/languages/json";
import typescript from "highlight.js/lib/languages/typescript";

/// Languages supported by static site examples.
export type ExampleLanguage = "JSON" | "TypeScript++" | "sh";

/// Syntax highlighter language for each visible language label.
const languages: Readonly<Record<ExampleLanguage, string>> = {
    JSON: "json",
    "TypeScript++": "typescript",
    sh: "bash",
};

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("json", json);
hljs.registerLanguage("typescript", typescript);

/// Highlight one trusted static example and return its rendered lines.
export function highlightExample(source: string, language: ExampleLanguage): readonly string[] {
    const highlighted = hljs.highlight(source, { language: languages[language] }).value;

    return highlighted.split("\n");
}
