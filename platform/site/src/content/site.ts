import type { SearchEntry } from "./search";
import type { ExampleLanguage } from "../site/highlight";

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// One code listing shown in a homepage example.
export type HomeCode = {
    /// The code text.
    code: string;

    /// The language label.
    language: ExampleLanguage;

    /// The file or terminal label.
    title: string;
};

/// One compact result shown below a homepage example.
export type HomeResult = {
    /// The result text.
    text: string;

    /// The result label.
    title: string;
};

/// One public product example.
export type HomeExample = {
    /// The action identifier and visible verb.
    action: string;

    /// The commercial-style headline attached to the chapter.
    claim: string;

    /// The short action description.
    description: string;

    /// The example source.
    source: HomeCode;

    /// The example result.
    result: HomeResult;

    /// The tools or systems replaced by this part of the stack.
    replacements: readonly string[];
};

/// The ordered Destack product examples.
export const homeExamples = [
    {
        action: "read",
        claim: "I can't believe it's not Node!",
        description: "standard TypeScript, TSX, Node, and Web code",
        source: {
            code: `function main() {
    console.log("Hello, Destack!");
}

main();`,
            language: "TypeScript++",
            title: "main.ds",
        },
        result: {
            text: "Hello, Destack!",
            title: "run",
        },
        replacements: ["TypeScript", "Rust", "Node.js"],
    },
    {
        action: "understand",
        claim: "One program. One model.",
        description: "one semantic model for the compiler, editor, and tools",
        source: {
            code: `export function choose(value: string): string {
    return value;
}

const selected = choose("web");`,
            language: "TypeScript++",
            title: "main.ds",
        },
        result: {
            text: `export function choose(value: string): string
main.ds:1:17`,
            title: "hover",
        },
        replacements: ["tsserver", "ESLint", "ast-grep"],
    },
    {
        action: "check",
        claim: "Types with teeth.",
        description: "strong types and project-defined rules",
        source: {
            code: `const port: uint8 = 300;
console.log(port);`,
            language: "TypeScript++",
            title: "main.ds",
        },
        result: {
            text: "✗ checked 1 module · 1 error",
            title: "diagnostic",
        },
        replacements: ["tsc", "ESLint", "Clippy"],
    },
    {
        action: "test",
        claim: "Every byte. Every cycle.",
        description: "correctness and exact cost measurement",
        source: {
            code: `import { expect, test } from "destack:test";

test("adds values", () => {
    expect(20 + 22).toBe(42);
});`,
            language: "TypeScript++",
            title: "math.test.ds",
        },
        result: {
            text: "test result: ok · 1 passed",
            title: "test",
        },
        replacements: ["Jest", "Vitest", "Criterion"],
    },
    {
        action: "simulate",
        claim: "Reality, now rewindable.",
        description: "the whole system backward, forward, or in slow motion",
        source: {
            code: `import { case as simulate } from "destack:simulation";

simulate("drains pending work", async (context) => {
    await context.runUntilIdle();
});`,
            language: "TypeScript++",
            title: "app.simulation.ds",
        },
        result: {
            text: "simulation result: ok · world idle",
            title: "simulation",
        },
        replacements: ["mocks", "staging", "replay debuggers"],
    },
    {
        action: "sandbox",
        claim: "Now with 100% less container.",
        description: "grant each package only the host access it needs",
        source: {
            code: `import { PolicyDecision, replacePolicy } from "destack:runtime";

replacePolicy(world, [{
    subject: { package: "@vendor/parser" },
    action: { action: "host.net.*" },
    target: { kind: "any" },
    decision: PolicyDecision.Deny,
}]);`,
            language: "TypeScript++",
            title: "policy.ds",
        },
        result: {
            text: "@vendor/parser   host.net.*   deny",
            title: "policy",
        },
        replacements: ["containers", "seccomp", "IAM"],
    },
    {
        action: "ship",
        claim: "Native means native.",
        description: "Web, VM, and AOT native from one source tree",
        source: {
            code: `destack build --target web
destack build --target native`,
            language: "sh",
            title: "terminal",
        },
        result: {
            text: `web     bundle
native  program`,
            title: "outputs",
        },
        replacements: ["bundlers", "cross-compilers", "packagers"],
    },
] as const satisfies readonly HomeExample[];

/// Searchable content outside the generated documentation and blog collections.
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        route: "/",
        text: `Destack the absurdly integrated open computing stack ${homeExamples
            .map((example) => `${example.action} ${example.description}`)
            .join(" ")}`,
        title: "Home",
    },
    ...homeExamples.map((example) => ({
        context: "destack.sh / actions",
        route: `/#${example.action}`,
        text: [example.claim, example.description, ...example.replacements].join(" "),
        title: example.action,
    })),
];
