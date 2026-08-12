/// Languages used by homepage technical listings.
export type HomeLanguage = "bash" | "destack" | "json" | "text";

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// The width of one homepage technical listing.
export type HomeListingWidth = "full" | "half";

/// One file, command, configuration, or result shown in a homepage chapter.
export type HomeListing = {
    /// The language used to highlight the listing.
    language: HomeLanguage;

    /// The file or terminal label.
    title: string;

    /// The listing text.
    text: string;

    /// The listing width within its chapter.
    width: HomeListingWidth;
};

/// One product example.
type HomeExampleShape = {
    /// The action identifier and visible verb.
    action: string;

    /// The primary product claim.
    claim: string;

    /// The supporting product description.
    description: string;

    /// The technical listings that demonstrate this part of the stack.
    listings: readonly HomeListing[];

    /// Familiar systems with comparable uses.
    like: readonly string[];
};

/// The ordered Destack product examples.
export const homeExamples = [
    {
        action: "read",
        claim: "Read and write what you already know.",
        description: "TypeScript-shaped source, TSX, familiar modules, and explicit effects.",
        listings: [
            {
                language: "destack",
                title: "src/receipt.ds",
                text: `import { log } from "destack:console";
import { HostError } from "destack:error/host";
import { args } from "destack:process";

type Order = {
    id: string;
    prices: uint32[];
};
type Receipt = { id: string; total: uint64 };

function receipt(order: Order): Receipt {
    return {
        id: order.id,
        total: order.prices.reduce((sum, price) => sum + price, 0),
    };
}

export function main(): Result<void, HostError> {
    const [id] = args()?;
    const order = { id, prices: [1200, 800, 450] };
    const result = receipt(order);

    return log(<output data-order={result.id}>{result.total}</output>);
}`,
                width: "full",
            },
        ],
        like: ["TypeScript", "TSX", "Node.js", "Web APIs"],
    },
    {
        action: "understand",
        claim: "Map out the exact, actual system.",
        description: "Inspect symbols, dependencies, calls, ownership, and effects without running the program.",
        listings: [
            {
                language: "destack",
                title: "src/orders.ds",
                text: `import { parseOrder } from "./order";
import { renderInvoice } from "./invoice";
import { spawn } from "destack:worker";

export const orders = spawn(parseOrder, { name: "orders" });
export const invoices = spawn(renderInvoice, { name: "invoices" });`,
                width: "full",
            },
            {
                language: "bash",
                title: "destack query",
                text: `destack query '$SPAWN($ENTRY, $$$OPTIONS)' src \\
    --where '$SPAWN == spawn' --capture ENTRY \\
    --with-filename --one-line`,
                width: "full",
            },
            {
                language: "text",
                title: "2 matches",
                text: `src/orders.ds:parseOrder
src/orders.ds:renderInvoice`,
                width: "full",
            },
        ],
        like: ["ast-grep", "Semgrep", "CodeQL", "C4"],
    },
    {
        action: "standardize",
        claim: "Enforce one standard approach, everywhere.",
        description: "Formatting, diagnostics, structured logs, traces, and metrics use stable shared forms.",
        listings: [
            {
                language: "json",
                title: "destack.json",
                text: `{
    "formatter": {
        "indentStyle": "space",
        "indentWidth": 4,
        "lineWidth": 100
    },
    "linter": { "enabled": true }
}`,
                width: "full",
            },
            {
                language: "destack",
                title: "src/checkout.ds",
                text: `import { logger } from "destack:telemetry";

const log = logger("checkout");

export function record(order: Order): void {
    log.info("order.confirmed", {
        orderId: order.id,
        total: order.total,
    });
}`,
                width: "full",
            },
        ],
        like: ["oxfmt", "Biome", "structlog", "OpenTelemetry"],
    },
    {
        action: "check",
        claim: "Check and lint, hard and strict.",
        description: "Typechecking, linting, and static analysis report through one diagnostic model.",
        listings: [
            {
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
        like: ["tsc", "tsgo", "ESLint", "Oxc"],
    },
    {
        action: "run",
        claim: "Run the same program across every host.",
        description: "Node, Electron, services, local tools, and isolated workers share one execution model.",
        listings: [
            {
                language: "destack",
                title: "src/thumbnail.ds",
                text: `import { AsyncResult } from "destack:async";
import { cache } from "./cache";
import { encode } from "./image";
import { WorkerError } from "destack:worker";
import { spawn } from "destack:worker";

type Job = { name: string; bytes: uint8[] };

const encoder = spawn(encode, {
    name: "thumbnail-encoder",
    stackSize: 2 * 1024 * 1024,
});

export async function render(job: shared Job): AsyncResult<void, WorkerError> {
    const thumbnail = await encoder.request(job)?;
    cache.set(thumbnail.name, thumbnail.bytes);
}`,
                width: "full",
            },
            {
                language: "bash",
                title: "same program, different hosts",
                text: `destack run src/thumbnail.ds --target node
destack run src/thumbnail.ds --target electron`,
                width: "full",
            },
        ],
        like: ["Node.js", "Electron", "Web Workers", "V8"],
    },
    {
        action: "test",
        claim: "Exercise code from every angle.",
        description: "Bounded executions, fixtures, assertions, isolation, and benchmarks use one runner.",
        listings: [
            {
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
                language: "text",
                title: "destack test",
                text: `PASS invoice.test.ds
  ✓ invoice totals › applies discounts before tax     1.4 ms
  ✓ invoice totals › keeps item order stable          0.6 ms

2 passed · 1 file · 2.0 ms`,
                width: "full",
            },
        ],
        like: ["Vitest", "Hypothesis", "QuickCheck"],
    },
    {
        action: "simulate",
        claim: "Reveal actual behavior under simulation.",
        description: "Explore unbounded inputs, schedules, time, entropy, host I/O, and faults deterministically.",
        listings: [
            {
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
        like: ["Antithesis", "FoundationDB DST", "Jepsen"],
    },
    {
        action: "analyze",
        claim: "Inspect and debug software in motion.",
        description: "Observe running software as flows, contention, allocation, critical paths, and cost.",
        listings: [
            {
                language: "bash",
                title: "observed run",
                text: `destack explain run checkout --scenario peak-hour --critical-path`,
                width: "full",
            },
            {
                language: "text",
                title: "checkout / peak-hour",
                text: `checkout                                      184 ms
├─ validateCart                                  2 ms
├─ reserveInventory                            121 ms
│  ├─ inventory.reserve     host.storage.write  38 ms
│  └─ retry backoff                 virtual      80 ms
└─ charge                                       61 ms
   └─ payments.authorize       host.net.connect  58 ms`,
                width: "full",
            },
            {
                language: "text",
                title: "observations",
                text: `critical path    checkout → reserveInventory → retry
host time        96 ms  / 52%
allocation       184 KiB across 42 objects
contention       inventory pool / 37 ms waiting`,
                width: "full",
            },
        ],
        like: ["OpenTelemetry", "Perfetto", "pprof", "Honeycomb"],
    },
    {
        action: "sandbox",
        claim: "Control exactly which bindings do what.",
        description: "Packages declare requirements; ordered rules grant exact host actions and resources.",
        listings: [
            {
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
        like: ["V8 Isolates", "Web Workers", "WASI"],
    },
    {
        action: "ship",
        claim: "Deploy directly to web, desktop, and server.",
        description: "The same source graph emits Web bundles, sandboxed bytecode, WASM, and native programs.",
        listings: [
            {
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
                language: "bash",
                title: "build matrix",
                text: `destack build --target web
destack build --target vm
destack build --target native`,
                width: "half",
            },
            {
                language: "text",
                title: "artifacts",
                text: `web       bundle    browser
vm        program   bytecode
native    program   native`,
                width: "half",
            },
        ],
        like: ["Vite", "Electron Builder", "wasm-pack", "Cargo"],
    },
] as const satisfies readonly HomeExampleShape[];

/// One public product example.
export type HomeExample = (typeof homeExamples)[number];
