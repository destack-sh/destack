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

    /// The short action description.
    description: string;

    /// The example source.
    source: HomeCode;

    /// The example result.
    result: HomeResult;
};

/// The ordered Destack product examples.
export const homeExamples = [
    {
        action: "write",
        description: "familiar TS / TSX, Node, and Web code",
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
    },
    {
        action: "inspect",
        description: "hover, completion, navigation, rename, and structural search",
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
    },
    {
        action: "sandbox",
        description: "grant each package only the host access it needs",
        source: {
            code: `{
    "policy": {
        "rules": [
            {
                "subject": { "package": "app" },
                "action": "fs.read",
                "resource": "app://config/**",
                "access": "allow"
            },
            {
                "subject": { "package": "@vendor/parser" },
                "action": "net.connect",
                "resource": "*",
                "access": "deny"
            }
        ]
    }
}`,
            language: "JSON",
            title: "destack.json",
        },
        result: {
            text: `app              fs.read      allow
@vendor/parser   net.connect  deny`,
            title: "policy",
        },
    },
    {
        action: "check",
        description: "strong typing and userland lints",
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
    },
    {
        action: "test",
        description: "every byte and cycle of your systems",
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
    },
    {
        action: "simulate",
        description: "the entire application end-to-end",
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
    },
    {
        action: "debug",
        description: "backward, in parallel or slow motion",
        source: {
            code: `import * as debug from "destack:debug";
import { Moment, WorldHandle } from "destack:runtime";

function rewind(world: WorldHandle, moment: Moment) {
    return debug.open(world).rewind(moment);
}`,
            language: "TypeScript++",
            title: "debug.ds",
        },
        result: {
            text: "rewound to the selected moment",
            title: "debugger",
        },
    },
    {
        action: "compile",
        description: "to sandboxed VM and true AOT native targets",
        source: {
            code: "destack build --target native",
            language: "sh",
            title: "terminal",
        },
        result: {
            text: "✓ built 1 module",
            title: "native",
        },
    },
    {
        action: "ship",
        description: "... web and native (really)",
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
    },
] as const satisfies readonly HomeExample[];

/// Searchable content outside the generated documentation and blog collections.
export const siteSearchEntries: readonly SearchEntry[] = [
    {
        context: "destack.sh",
        route: "/",
        text: `Destack the absurdly integrated standardized computing stack ${homeExamples
            .map((example) => `${example.action} ${example.description}`)
            .join(" ")}`,
        title: "Home",
    },
    ...homeExamples.map((example) => ({
        context: "destack.sh / actions",
        route: `/#${example.action}`,
        text: example.description,
        title: example.action,
    })),
    {
        context: "footer",
        route: "/",
        text: "copyright symbol industries",
        title: "Symbol Industries",
    },
];
