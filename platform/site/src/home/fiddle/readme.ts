import { marked } from "marked";

import type { ExampleArea, ExampleCategory, ExampleTopic } from "../../examples";

type RenderedMarkdown = {
    html: string;
    source: string;
};

const areaNotes: Record<string, string> = {
    language:
        "Surface language examples for the type system, expression model, and memory rules.",
    runtime:
        "Runtime examples for modules, conditions, import metadata, policies, and the built-in test tools.",
};

const categoryNotes: Record<string, string> = {
    "language/types":
        "Precise values, nominal wrappers, structural records, generics, constraints, and static members.",
    "language/expressions":
        "Expression-oriented control flow, pattern matching, managed scopes, TSX, comptime code, and macros.",
    "language/memory":
        "Address spaces, ownership, borrowing, lifetimes, disposal, and explicit unsafe boundaries.",
    "runtime/modules":
        "Package graph behavior, conditional entry points, import metadata, data modules, and policy checks.",
    "runtime/tests":
        "Tests, fuzzing, benchmarks, and simulation as first-class runtime surfaces.",
};

const topicNotes: Record<string, string> = {
    "types/primitives": "fixed-width integers, floats, strings, booleans, and checked value refinements",
    "types/intervals": "integer intervals that reject values outside declared bounds",
    "types/newtypes": "nominal wrappers that keep storage cheap while preventing accidental mixing",
    "types/extensions": "extension methods and static extension members on existing types",
    "types/enums": "closed named values with exhaustiveness across static analysis and lowering",
    "types/structs": "records with clear field layout, initialization, and readable construction",
    "types/tuples": "fixed positional values for compact local grouping",
    "types/slices": "borrowed views over contiguous values",
    "types/arrays": "fixed-size arrays with length in the type",
    "types/readonly": "deep readonly views and diagnostics for illegal mutation",
    "types/generics": "type parameters, constant parameters, and monomorphized lowering",
    "types/constraints": "capability-style bounds over generic values",
    "types/associated": "associated constants and members tied to a type",
    "types/reflection": "compile-time type facts without runtime shape guessing",
    "types/tagged-unions": "closed variants with compact tags and exhaustive handling",
    "types/static": "static type members for layout, parsing, construction, and constants",
    "types/any": "diagnostics for unsafe escape hatches in native modules",
    "expressions/blocks": "block expressions that return values from local control flow",
    "expressions/patterns": "destructuring in lets, parameters, and local bindings",
    "expressions/let-else": "early exits when a pattern does not match",
    "expressions/match": "exhaustive branching over results, variants, and guards",
    "expressions/is": "narrowing checks that feed the type system",
    "expressions/loops": "loops that preserve local invariants and typed control flow",
    "expressions/using": "managed resources with deterministic cleanup",
    "expressions/ranges": "inclusive and exclusive ranges for iteration and slicing",
    "expressions/overloads": "operator and function overloads with explicit resolution",
    "expressions/errors": "result handling diagnostics when fallible values are not opened",
    "expressions/tsx": "typed component syntax for UI surfaces",
    "expressions/decorators": "metadata hooks that stay visible to analysis",
    "expressions/module": "module-level declarations and exported values",
    "expressions/comptime": "compile-time evaluation for layout, constants, and generated tables",
    "expressions/macros": "syntax expansion that still renders inspectable source",
    "expressions/captures": "closures and captured values with explicit lowering",
    "memory/space": "address spaces and storage placement",
    "memory/ownership": "single-owner values and transfer rules",
    "memory/borrowing": "shared and exclusive borrow diagnostics",
    "memory/lifetimes": "borrowed values that cannot outlive their source",
    "memory/drop": "value destruction for owned resources",
    "memory/dispose": "scope-bound external resources",
    "memory/unsafe": "explicit unsafe regions for operations the checker cannot prove",
    "memory/polymorphism": "owned, borrowed, and erased values across dispatch boundaries",
    "runtime/modules": "modules, roles, imports, and package entry points",
    "runtime/conditions": "target and stage conditions in the module graph",
    "runtime/import-meta": "typed metadata exposed to compiled modules",
    "runtime/data-modules": "JSON, text, bytes, and other non-code imports",
    "runtime/policy": "runtime policy checks that stay tied to source modules",
    "runtime/test": "ordinary tests as native runtime declarations",
    "runtime/fuzz": "generators, shrinking, and property-style checks",
    "runtime/bench": "benchmark loops with declared throughput units",
    "runtime/simulation": "faulted worlds for distributed and stateful systems",
};

marked.use({ gfm: true });

export function readmeForArea(area: ExampleArea): RenderedMarkdown {
    const key = area.label.toLowerCase();
    const rows = area.categories
        .map((category) => {
            const categoryKey = `${key}/${category.label.toLowerCase()}`;
            const label = `${category.label.toLowerCase()}/`;
            const href = `${category.label.toLowerCase()}/README.md`;

            return `| [\`${label}\`](${href}) | ${categoryNotes[categoryKey]} |`;
        })
        .join("\n");

    const source = `# ${key}/

${areaNotes[key]}

| folder | covers |
| --- | --- |
${rows}
`;

    return { html: renderMarkdown(source), source };
}

export function readmeForCategory(
    area: ExampleArea,
    category: ExampleCategory,
): RenderedMarkdown {
    const areaKey = area.label.toLowerCase();
    const categoryKey = category.label.toLowerCase();
    const rows = category.topics.map(topicRow).join("\n");

    const source = `# ${categoryKey}/

${categoryNotes[`${areaKey}/${categoryKey}`]}

| file | covers |
| --- | --- |
${rows}
`;

    return { html: renderMarkdown(source), source };
}

function topicRow(topic: ExampleTopic) {
    const label = `${topic.label}.ds`;

    return `| [\`${label}\`](${label}) | ${topicNotes[topic.key]} |`;
}

function renderMarkdown(markdown: string) {
    return marked.parse(markdown, { async: false, gfm: true }) as string;
}
