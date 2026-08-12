/// Languages used by homepage technical listings.
export type HomeLanguage = "bash" | "destack" | "json" | "text";

/// The public installation command.
export const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// One editor or output shown in a homepage chapter.
export type HomeListing = {
    /// The language used to highlight the listing.
    language: HomeLanguage;

    /// The file or terminal label.
    title: string;

    /// The listing text.
    text: string;
};

/// One product example.
type HomeExampleShape = {
    /// The action identifier and visible verb.
    action: string;

    /// The primary product claim.
    claim: string;

    /// The supporting product description.
    description: string;

    /// The files and configuration shown in the editor area.
    editors: readonly HomeListing[];

    /// The command and result shown in the output area, when present.
    output: HomeListing | undefined;

    /// Familiar systems with comparable uses.
    like: readonly string[];
};

/// The ordered Destack product examples.
export const homeExamples = [
    {
        action: "read",
        claim: "Read and write what you already know.",
        description:
            "Meet Relay: a small webhook service built with typed routes, Web APIs, TSX, and workers.",
        editors: [
            {
                language: "destack",
                title: "src/server.ds",
                text: `import { serve } from "destack:http";
import { spawn } from "destack:worker";

import { RelayDashboard } from "./dashboard";
import { deliver } from "./delivery";
import { receive } from "./event";
import { subscriptions } from "./subscription";

const deliveries = spawn(deliver, { name: "deliveries" });
const dashboard = <RelayDashboard title="Relay activity" />;

serve({
    routes: {
        "/": dashboard,
        "/hooks/:source": {
            async POST(request) {
                const event = await receive(request)?;
                deliveries.post({ event, endpoint: subscriptions[event.source] });

                return Response.json({ accepted: event.id }, { status: 202 });
            },
        },
    },
});`,
            },
        ],
        output: undefined,
        like: ["TypeScript", "Rust", "Node.js", "Web APIs"],
    },
    {
        action: "understand",
        claim: "Map out the exact, actual system.",
        description:
            "Ask the source where events can leave the process and get precise structural answers.",
        editors: [
            {
                language: "destack",
                title: "src/delivery.ds",
                text: `import { logger } from "destack:telemetry";

import { Delivery } from "./model";

const log = logger("relay.delivery");

export async function deliver(job: Delivery): Promise<void> {
    const response = await fetch(job.endpoint.url, {
        method: "POST",
        headers: job.headers,
        body: JSON.stringify(job.event),
    });

    log.info("delivery.completed", {
        eventId: job.event.id,
        status: response.status,
    });
}`,
            },
        ],
        output: {
            language: "bash",
            title: "destack query",
            text: `$ destack query 'fetch($URL, $$$OPTIONS)' src \\
    --capture URL --with-filename --one-line

1 match
src/delivery.ds:job.endpoint.url`,
        },
        like: ["ast-grep", "Semgrep", "CodeQL", "C4"],
    },
    {
        action: "standardize",
        claim: "Enforce one standard approach, everywhere.",
        description:
            "The same formatter, diagnostics, logs, traces, and metrics describe the whole relay.",
        editors: [
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
            },
            {
                language: "destack",
                title: "src/delivery.ds",
                text: `import { logger, tracer } from "destack:telemetry";

import { Delivery } from "./model";
import { send } from "./request";

const log = logger("relay.delivery");
const trace = tracer();

export async function deliver(job: Delivery): Promise<void> {
    await trace.spanAsync("delivery.attempt", async (span) => {
        span.addFields({ eventId: job.event.id, endpoint: job.endpoint.id });
        const response = await send(job);
        log.info("delivery.completed", { status: response.status });
    });
}`,
            },
        ],
        output: undefined,
        like: ["oxfmt", "Biome", "structlog", "OpenTelemetry"],
    },
    {
        action: "check",
        claim: "Check and lint, hard and strict.",
        description:
            "Typechecking, linting, and static analysis catch invalid delivery behavior together.",
        editors: [
            {
                language: "destack",
                title: "src/delivery.ds",
                text: `newtype Attempts = 1..=8;

type Delivery = {
    event: Event;
    endpoint: Endpoint;
    attempt: Attempts;
};

export async function deliver(job: Delivery): Promise<void> {
    const response = await fetch(job.endpoint.url, {
        method: "POST",
        body: JSON.stringify(job.event),
    });

    response.json();
    retry({ ...job, attempt: job.attempt + 1 });
}`,
            },
        ],
        output: {
            language: "text",
            title: "destack check",
            text: `$ destack check

warning[unused-result]: result of \`response.json()\` is ignored
 ──▶ src/delivery.ds:16:5

error[not-assignable]: \`Attempts + 1\` may exceed \`1..=8\`
 ──▶ src/delivery.ds:17:30
  │ prove a bound or handle the exhausted retry budget`,
        },
        like: ["tsc", "tsgo", "ESLint", "Oxc"],
    },
    {
        action: "run",
        claim: "Run the same program across every host.",
        description:
            "One event moves from an HTTP request into a named delivery worker on any supported host.",
        editors: [
            {
                language: "destack",
                title: "src/server.ds",
                text: `import { serve } from "destack:http";
import { spawn } from "destack:worker";

import { deliver } from "./delivery";
import { receive } from "./event";
import { subscriptions } from "./subscription";

const deliveries = spawn(deliver, { name: "deliveries" });

serve({
    routes: {
        "/hooks/:source": {
            async POST(request) {
                const event = await receive(request)?;
                deliveries.post({ event, endpoint: subscriptions[event.source] });

                return Response.json({ accepted: event.id }, { status: 202 });
            },
        },
    },
});`,
            },
        ],
        output: {
            language: "bash",
            title: "relay / local",
            text: `$ destack run relay
listening   http://127.0.0.1:8787
accepted    evt_01JQ8M → deliveries
delivered   evt_01JQ8M → acme / 202`,
        },
        like: ["Node.js", "Bun", "Electron", "Workers"],
    },
    {
        action: "test",
        claim: "Exercise code from every angle.",
        description:
            "Bounded tests use the same Request, Response, worker, and assertion models as the program.",
        editors: [
            {
                language: "destack",
                title: "delivery.test.ds",
                text: `import { expect, test } from "destack:test";

import { deliver } from "./delivery";
import { receive } from "./event";
import { fixture } from "./fixture";

test("forwards the signed event unchanged", async () => {
    const event = fixture.event({ id: "evt_01JQ8M" });
    const endpoint = fixture.endpoint();
    const response = await deliver({ event, endpoint, attempt: 1 });

    expect(response.status).toBe(202);
    expect(endpoint.requests[0].headers.get("x-relay-event")).toBe(event.id);
    expect(await endpoint.requests[0].json()).toEqual(event);
});

test("rejects an invalid signature", async () => {
    const response = await receive(fixture.request({ signature: "invalid" }));

    expect(response.status).toBe(401);
});`,
            },
        ],
        output: {
            language: "text",
            title: "destack test",
            text: `$ destack test

PASS delivery.test.ds
  ✓ forwards the signed event unchanged          1.8 ms
  ✓ rejects an invalid signature                 0.4 ms

2 passed · 1 file · 2.2 ms`,
        },
        like: ["Vitest", "Hypothesis", "QuickCheck"],
    },
    {
        action: "simulate",
        claim: "Reveal actual behavior under simulation.",
        description:
            "Control time and host failures to explore every schedule, retry, and external operation.",
        editors: [
            {
                language: "destack",
                title: "relay.simulation.ds",
                text: `import { binding, scenario, simulation } from "destack:simulation";
import { expect } from "destack:test";

import { fixture, relay } from "./fixture";

simulation.case("retries one delivery timeout", async (context) => {
    context.fault(
        binding.binding("destack.net.socket.connect"),
        { kind: "operation.timeout" },
        scenario.once(),
    );

    await relay.accept(fixture.event({ id: "evt_01JQ8M" }));
    await context.runUntilIdle();

    expect(relay.delivery("evt_01JQ8M").attempts).toBe(2);
    expect(relay.delivery("evt_01JQ8M").status).toBe("delivered");
});`,
            },
        ],
        output: {
            language: "text",
            title: "destack test relay.simulation.ds",
            text: `$ destack test relay.simulation.ds

PASS relay.simulation.ds / seed 9f2c
  injected    destack.net.socket.connect / timeout
  advanced    virtual time / 1.00 s
  delivered   evt_01JQ8M / attempt 2

1 passed · 2 schedules · replay 9f2c`,
        },
        like: ["Antithesis", "FoundationDB DST", "Jepsen"],
    },
    {
        action: "analyze",
        claim: "Inspect and debug software in motion.",
        description:
            "Follow the same event through workers, bindings, retries, allocations, and virtual time.",
        editors: [],
        output: {
            language: "text",
            title: "observed run",
            text: `$ destack explain run relay --event evt_01JQ8M --critical-path

evt_01JQ8M / critical path
receive                                                   1.1 ms
├─ verify signature                                       0.3 ms
├─ enqueue → worker:deliveries                            0.2 ms
└─ deliver → https://api.acme.test/hooks
   ├─ attempt 1    host.net.connect / timeout             500 ms
   ├─ retry        virtual time                            1.0 s
   └─ attempt 2    202 Accepted                            42 ms

result       delivered / 2 attempts
host time    542 ms
allocation   38 KiB / 14 objects`,
        },
        like: ["OpenTelemetry", "Perfetto", "pprof", "Honeycomb"],
    },
    {
        action: "sandbox",
        claim: "Control exactly which bindings do what.",
        description:
            "Verification may read its signing key; delivery may use the network; no other host access is granted.",
        editors: [
            {
                language: "json",
                title: "destack.json",
                text: `{
    "policy": {
        "requires": [
            { "action": "fs.read", "resource": "app://secrets/relay-signing-key" },
            { "action": "net.connect", "resource": "https://*.acme.test/**" }
        ],
        "rules": [
            {
                "subject": { "package": "@relay/verify" },
                "action": "fs.read",
                "resource": "app://secrets/relay-signing-key",
                "access": "allow"
            },
            {
                "subject": { "package": "@relay/delivery" },
                "action": "net.connect",
                "resource": "https://*.acme.test/**",
                "access": "allow"
            }
        ]
    }
}`,
            },
        ],
        output: undefined,
        like: ["Deno Permissions", "WASI", "V8 Isolates"],
    },
    {
        action: "ship",
        claim: "Deploy directly to web, desktop, and server.",
        description:
            "Build the activity view, inspector, and relay service from the same source graph.",
        editors: [
            {
                language: "json",
                title: "destack.json",
                text: `{
    "targets": {
        "dashboard": {
            "entry": ["src/dashboard.ds"],
            "output": "bundle",
            "host": "browser"
        },
        "inspector": {
            "entry": ["src/inspector.ds"],
            "output": "program",
            "code": ["wasm"],
            "host": "native"
        },
        "relay": {
            "entry": ["src/server.ds"],
            "output": "program",
            "code": ["native"],
            "host": "native"
        }
    }
}`,
            },
        ],
        output: {
            language: "text",
            title: "build",
            text: `$ destack build --target dashboard
$ destack build --target inspector
$ destack build --target relay

dashboard   browser   JS bundle
inspector   desktop   WASM program
relay       server    native program`,
        },
        like: ["Vite", "Electron Builder", "wasm-pack", "Cargo"],
    },
] as const satisfies readonly HomeExampleShape[];

/// One public product example.
export type HomeExample = (typeof homeExamples)[number];
