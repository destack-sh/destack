import type { SearchEntry } from "./search";

/// Languages used by homepage technical artifacts.
export type HomeLanguage = "bash" | "destack" | "json" | "text";

/// Visible label for each homepage source language.
export const homeLanguageLabels: Readonly<Record<HomeLanguage, string>> = {
    bash: "sh",
    destack: "TypeScript++",
    json: "JSON",
    text: "output",
};

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// The purpose of one homepage technical artifact.
export type HomeArtifactKind = "command" | "configuration" | "output" | "source";

/// The width of one homepage technical artifact.
export type HomeArtifactWidth = "full" | "half";

/// One source, command, configuration, or output shown in a homepage chapter.
export type HomeArtifact = {
    /// The artifact purpose.
    kind: HomeArtifactKind;

    /// The parser used for the listing.
    language: HomeLanguage;

    /// The file or terminal label.
    title: string;

    /// The artifact text.
    text: string;

    /// The artifact width within its chapter dossier.
    width: HomeArtifactWidth;
};

/// One product example.
type HomeExampleShape = {
    /// The action identifier and visible verb.
    action: string;

    /// The primary product claim.
    claim: string;

    /// The supporting product description.
    description: string;

    /// The technical artifacts that demonstrate this part of the stack.
    artifacts: readonly HomeArtifact[];

    /// The capabilities covered by this part of the stack.
    scope: readonly string[];
};

/// The ordered Destack product examples.
export const homeExamples = [
    {
        action: "read",
        claim: "Write the software you already know.",
        description: "TypeScript-shaped source, TSX, familiar modules, and explicit shared memory.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "src/receipt.ds",
                text: `import { log } from "destack:console";
import { args } from "destack:process";
import { spawn } from "destack:worker";

type Order = {
    id: string;
    prices: uint32[];
};
type Receipt = { id: string; total: uint64 };

function total(order: shared Order): shared Receipt {
    return {
        id: order.id,
        total: order.prices.reduce((sum, price) => sum + price, 0),
    };
}

const receipts = spawn(total, { name: "receipts" });

export async function main(): Promise<void> {
    const [id] = args()?;
    const order: shared Order = { id, prices: [1200, 800, 450] };
    const receipt = await receipts.request(order)?;

    log(<output data-order={receipt.id}>{receipt.total}</output>)?;
}`,
                width: "full",
            },
        ],
        scope: ["TypeScript", "TSX", "workers"],
    },
    {
        action: "understand",
        claim: "Ask the source real questions.",
        description: "Structural queries use the compiler's parser instead of regular expressions.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "src/orders.ds",
                text: `import { spawn } from "destack:worker";

declare function parseOrder(source: shared string): shared Order;
declare function renderInvoice(order: shared Order): shared Invoice;

export const orders = spawn(parseOrder, { name: "orders" });
export const invoices = spawn(renderInvoice, { name: "invoices" });`,
                width: "full",
            },
            {
                kind: "command",
                language: "bash",
                title: "destack query",
                text: `destack query '$SPAWN($ENTRY, $$$OPTIONS)' src \\
    --where '$SPAWN == spawn' --capture ENTRY \\
    --with-filename --one-line`,
                width: "half",
            },
            {
                kind: "output",
                language: "text",
                title: "2 matches",
                text: `src/orders.ds:parseOrder
src/orders.ds:renderInvoice`,
                width: "half",
            },
        ],
        scope: ["patterns", "symbols", "rewrites"],
    },
    {
        action: "check",
        claim: "Check the complete program once.",
        description: "Types, ownership, control flow, and project lints report through one diagnostic model.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "src/server.ds",
                text: `newtype Port = 1..=65535;

struct ServerOptions {
    enabled: boolean;
    port: Port;
    retries: uint8;
}

export const server = ServerOptions {
    enabled: true,
    port: 70000,
    retries: 300,
};`,
                width: "full",
            },
            {
                kind: "output",
                language: "text",
                title: "destack check",
                text: `warning[boolean-prefix]: boolean value \`enabled\` needs a predicate prefix
 ──▶ src/server.ds:4:5
  │ enabled: boolean;
  │ ^^^^^^^ rename to \`isEnabled\`

error[not-assignable]: type \`70000\` is not assignable to type \`Port\`
 ──▶ src/server.ds:11:11

error[not-assignable]: type \`300\` is not assignable to type \`uint8\`
 ──▶ src/server.ds:12:14`,
                width: "full",
            },
        ],
        scope: ["types", "ownership", "lints"],
    },
    {
        action: "run",
        claim: "One runtime. Every application.",
        description: "Node, Electron, services, local tools, and isolated workers share one execution model.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "src/thumbnail.ds",
                text: `import { log } from "destack:console";
import { spawn } from "destack:worker";

type Job = { name: string; bytes: uint8[] };
type Thumbnail = { name: string; bytes: uint8[] };

function encode(job: shared Job): shared Thumbnail {
    return {
        name: job.name,
        bytes: resize(job.bytes, { width: 320, format: "webp" }),
    };
}

const encoder = spawn(encode, {
    name: "thumbnail-encoder",
    stackSize: 2 * 1024 * 1024,
});

export async function render(job: shared Job): Promise<void> {
    const thumbnail = await encoder.request(job)?;
    log("encoded", thumbnail.name, thumbnail.bytes.length)?;
}`,
                width: "full",
            },
            {
                kind: "command",
                language: "bash",
                title: "same program, different hosts",
                text: `destack run src/thumbnail.ds --target node
destack run src/thumbnail.ds --target electron`,
                width: "full",
            },
        ],
        scope: ["Node.js", "Electron", "workers"],
    },
    {
        action: "test",
        claim: "Keep the test ergonomics.",
        description: "Suites, assertions, isolation, retries, and benchmarks use one project-aware runner.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "invoice.test.ds",
                text: `import { describe, expect, test } from "destack:test";

describe("invoice totals", () => {
    test("applies discounts before tax", () => {
        const invoice = total({
            items: [{ price: 2000, quantity: 2 }],
            discount: 500,
            taxRate: 0.2,
        });

        expect(invoice.subtotal).toBe(4000);
        expect(invoice.total).toBe(4200);
    });

    test("keeps item order stable", () => {
        expect(sortItems(["b", "a"])).toEqual(["b", "a"]);
    });
});`,
                width: "full",
            },
            {
                kind: "output",
                language: "text",
                title: "destack test",
                text: `PASS invoice.test.ds
  ✓ invoice totals › applies discounts before tax     1.4 ms
  ✓ invoice totals › keeps item order stable          0.6 ms

2 passed · 1 file · 2.0 ms`,
                width: "full",
            },
        ],
        scope: ["tests", "isolation", "benchmarks"],
    },
    {
        action: "simulate",
        claim: "Break the world on purpose.",
        description: "Generate inputs and control time, entropy, host I/O, and faults deterministically.",
        artifacts: [
            {
                kind: "source",
                language: "destack",
                title: "checkout.simulation.ds",
                text: `import * as simulation from "destack:simulation";
import { expect } from "destack:test";

simulation.case("retries one inventory timeout", async (context) => {
    context.fault(
        simulation.binding.binding("inventory.reserve"),
        { kind: "operation.timeout" },
        simulation.scenario.once(),
    );

    const result = await checkout(order);
    await context.runUntilIdle();

    expect(result.status).toBe("confirmed");
});`,
                width: "full",
            },
            {
                kind: "source",
                language: "destack",
                title: "total.fuzz.ds",
                text: `import { case, Generator } from "destack:fuzz";
import { expect } from "destack:test";

const quantities = Generator.array({
    element: Generator.uint32(0..=100),
    maxLength: 64,
});

case("total never becomes negative", quantities, (values) => {
    expect(total(values)).toBeGreaterThan(-1);
});`,
                width: "full",
            },
        ],
        scope: ["generation", "faults", "virtual time"],
    },
    {
        action: "analyze",
        claim: "See the system, not just files.",
        description: "Explain semantic call graphs, binding effects, critical paths, allocation, and cost.",
        artifacts: [
            {
                kind: "command",
                language: "bash",
                title: "semantic change",
                text: `destack explain change main..HEAD --entry checkout`,
                width: "half",
            },
            {
                kind: "output",
                language: "text",
                title: "main..HEAD",
                text: `checkout
├─ validateCart                 pure
├─ reserveInventory            + call
│  └─ inventory.reserve       host.storage.write
└─ charge
   └─ payments.authorize       host.net.connect`,
                width: "half",
            },
            {
                kind: "output",
                language: "text",
                title: "destack build --timings",
                text: `parse      ████████                         18.2 ms
check              ████████████             31.7 ms
optimize                       ████████████  29.4 ms
emit                                       ████  10.1 ms
total                                                89.4 ms`,
                width: "full",
            },
        ],
        scope: ["graphs", "causality", "cost"],
    },
    {
        action: "sandbox",
        claim: "Authority is project configuration.",
        description: "Packages declare requirements; ordered rules grant exact host actions and resources.",
        artifacts: [
            {
                kind: "configuration",
                language: "json",
                title: "destack.json",
                text: `{
    "policy": {
        "requires": [
            {
                "domain": "runtime",
                "action": "host.fs.read",
                "resource": "assets/**"
            }
        ],
        "rules": [
            {
                "domain": "runtime",
                "subject": {
                    "package": { "patterns": ["@app/*"] }
                },
                "action": "host.fs.read",
                "resource": "assets/**",
                "access": "allow"
            },
            {
                "domain": "runtime",
                "subject": {
                    "package": { "patterns": ["@vendor/*"] }
                },
                "action": "host.net.*",
                "resource": "*",
                "access": "deny"
            }
        ]
    }
}`,
                width: "full",
            },
        ],
        scope: ["packages", "actions", "resources"],
    },
    {
        action: "ship",
        claim: "Choose output, not another stack.",
        description: "The same source graph emits Web bundles, sandboxed bytecode, WASM, and native programs.",
        artifacts: [
            {
                kind: "configuration",
                language: "json",
                title: "destack.json",
                text: `{
    "targets": {
        "web": {
            "entry": ["src/app.ds"],
            "output": "bundle",
            "host": "browser"
        },
        "vm": {
            "entry": ["src/app.ds"],
            "output": "program",
            "code": ["bytecode"],
            "host": "native"
        },
        "native": {
            "entry": ["src/app.ds"],
            "output": "program",
            "code": ["native"],
            "host": "native",
            "compiler": { "optimize": "o2" }
        }
    }
}`,
                width: "full",
            },
            {
                kind: "command",
                language: "bash",
                title: "build matrix",
                text: `destack build --target web
destack build --target vm
destack build --target native`,
                width: "half",
            },
            {
                kind: "output",
                language: "text",
                title: "artifacts",
                text: `web       bundle    browser
vm        program   bytecode
native    program   native`,
                width: "half",
            },
        ],
        scope: ["Web", "bytecode", "native"],
    },
] as const satisfies readonly HomeExampleShape[];

/// One public product example.
export type HomeExample = (typeof homeExamples)[number];

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
        text: [example.claim, example.description, ...example.scope].join(" "),
        title: example.action,
    })),
];
