import type { ESTree, Rule } from "@oxlint/plugins";
import { isEnvironmentFile } from "./package.ts";

/** The host modules each effect comes from, and the environment member taking their place. */
const MODULES = new Map([
    ["node:fs", "environment.files"],
    ["node:fs/promises", "environment.files"],
    ["node:child_process", "environment.process.spawn"],
    ["node:os", "environment.platform"],
    ["fs", "environment.files"],
    ["fs/promises", "environment.files"],
    ["child_process", "environment.process.spawn"],
    ["os", "environment.platform"],
    ["node:net", "environment.network.connect"],
    ["node:http", "environment.server.http"],
    ["node:https", "environment.network.fetch"],
    ["node:tls", "environment.network.connect"],
    ["node:dgram", "environment.network"],
    ["node:worker_threads", "environment.workers"],
    ["node:crypto", "environment.crypto, or Digest.of for hashes"],
    ["net", "environment.network.connect"],
    ["http", "environment.server.http"],
    ["https", "environment.network.fetch"],
    ["tls", "environment.network.connect"],
    ["dgram", "environment.network"],
    ["worker_threads", "environment.workers"],
    ["crypto", "environment.crypto, or Digest.of for hashes"],
]);

/** The host globals whose every member is an effect, and the environment taking their place. */
const HOSTS = new Map([
    ["process", "environment.process"],
    ["Bun", "the environment"],
    ["localStorage", "environment.storage"],
    ["sessionStorage", "environment.storage"],
    ["location", "environment.navigation.currentEntry"],
    ["navigator", "the environment"],
    ["history", "environment.navigation"],
]);

/** The host globals' members that compute without effects: Bun's parser. */
const PURE_MEMBERS = new Set(["Bun.Transpiler"]);

/** The names of the global object, through which every host global is also reached. */
const GLOBAL_OBJECTS = new Set(["globalThis", "window", "self"]);

/** The uuid package's generators reading host clocks or randomness, and their replacements. */
const UUIDS = new Map([
    ["v1", "Uuid.v7(environment)"],
    ["v4", "Uuid.v4(environment.random)"],
    ["v6", "Uuid.v7(environment)"],
    ["v7", "Uuid.v7(environment)"],
]);

/** The host globals' members each effect comes from, and the member taking their place. */
const MEMBERS = new Map([
    ["process.env", "environment.process.variables"],
    ["process.argv", "environment.process.arguments"],
    ["process.cwd", "environment.process.workingDirectory"],
    ["process.pid", "environment.process.id"],
    ["process.exit", "environment.process.exit"],
    ["process.on", "environment.process.on"],
    ["process.stdin", "environment.process.stdin"],
    ["process.stdout", "environment.process.stdout"],
    ["process.stderr", "environment.process.stderr"],
    ["process.platform", "environment.platform.os"],
    ["process.arch", "environment.platform.architecture"],
    ["Bun.spawn", "environment.process.spawn"],
    ["Bun.serve", "environment.server.http"],
    ["Bun.listen", "environment.server.tcp"],
    ["Bun.connect", "environment.network.connect"],
    ["Bun.file", "environment.files"],
    ["Bun.write", "environment.files"],
    ["Bun.sleep", "environment.clock.sleep"],
    ["Bun.env", "environment.process.variables"],
    ["Bun.argv", "environment.process.arguments"],
    ["Bun.nanoseconds", "environment.clock.now"],
    ["globalThis.fetch", "environment.network.fetch"],
    ["crypto.subtle", "environment.crypto"],
    ["navigator.clipboard", "environment.clipboard"],
    ["navigator.onLine", "environment.connectivity.isOnline"],
    ["navigator.mediaDevices", "environment.capture"],
    ["navigator.geolocation", "environment.geolocation"],
    ["navigator.share", "environment.share.share"],
    ["navigator.canShare", "environment.share.canShare"],
    ["navigator.vibrate", "environment.vibration.vibrate"],
    ["navigator.permissions", "environment.permissions"],
    ["navigator.language", "environment.locale"],
    ["navigator.languages", "environment.locale"],
    ["crypto.randomUUID", "Uuid.v4(environment.random)"],
    ["crypto.getRandomValues", "environment.random.bytes"],
    ["Math.random", "Random.fraction(environment.random)"],
    ["Date.now", "environment.clock.now"],
    ["performance.now", "environment.clock.now"],
    ["trace.getTracer", "environment.telemetry.tracers"],
    ["trace.getTracerProvider", "environment.telemetry.tracers"],
    ["trace.setGlobalTracerProvider", "environment.telemetry.tracers"],
    ["metrics.getMeter", "environment.telemetry.meters"],
    ["metrics.getMeterProvider", "environment.telemetry.meters"],
    ["metrics.setGlobalMeterProvider", "environment.telemetry.meters"],
    ["logs.getLogger", "environment.telemetry.loggers"],
    ["logs.getLoggerProvider", "environment.telemetry.loggers"],
    ["logs.setGlobalLoggerProvider", "environment.telemetry.loggers"],
    ["context.active", "environment.telemetry.context.active"],
    ["context.with", "environment.telemetry.context.with"],
    ["context.setGlobalContextManager", "environment.telemetry.context"],
    ["propagation.inject", "environment.telemetry.propagator.inject"],
    ["propagation.extract", "environment.telemetry.propagator.extract"],
    ["propagation.setGlobalPropagator", "environment.telemetry.propagator"],
    ["trace.getActiveSpan", "trace.getSpan(environment.telemetry.context.active())"],
]);

/** The host constructors each effect comes from, and the environment member taking their place. */
const CONSTRUCTORS = new Map([
    ["CompressionStream", "environment.compression.compress"],
    ["DecompressionStream", "environment.compression.decompress"],
    ["Worker", "environment.workers.start"],
    ["WebSocket", "environment.network.websocket"],
    ["EventSource", "a streamed environment.network.fetch"],
    ["BroadcastChannel", "a capability of the environment"],
]);

/** The formatting methods that read the host's locale without one. */
const FORMATTING = new Set(["toLocaleString", "toLocaleDateString", "toLocaleTimeString"]);

/** The host functions, timers and fetch, and the environment member taking their place. */
const FUNCTIONS = new Map([
    ["setTimeout", "environment.clock.sleep"],
    ["setInterval", "environment.clock.sleep"],
    ["fetch", "environment.network.fetch"],
    ["matchMedia", "environment.display.media"],
    ["requestAnimationFrame", "environment.display.frame"],
    ["cancelAnimationFrame", "environment.display.frame's signal"],
    ["requestIdleCallback", "environment.display.idle"],
    ["cancelIdleCallback", "environment.display.idle's signal"],
]);

/** Reject reaching the host's effects outside the environment's host modules. */
export const noAmbientEffect: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "take every effect from the injected environment" },
        messages: { effect: "take {{name}} from the environment: use {{replacement}}" },
    },
    create(context) {
        // leave the environment's own host modules
        if (isEnvironmentFile(context.filename)) {
            return {};
        }

        /** Report whether a name resolves to a declaration of the file other than an import. */
        function isLocal(node: ESTree.Node, name: string): boolean {
            // walk the scopes outward to the name's declaration
            let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                context.sourceCode.getScope(node);
            while (scope) {
                const definition = scope.set.get(name)?.defs[0];
                if (definition !== undefined) {
                    return definition.type !== "ImportBinding";
                }
                scope = scope.upper;
            }

            return false;
        }

        return {
            ImportDeclaration(node) {
                // report a host module
                const name = node.source.value;
                const replacement = MODULES.get(name);
                if (replacement !== undefined) {
                    context.report({ node, messageId: "effect", data: { name, replacement } });
                }

                // report each generator of the uuid package
                const imported = name === "uuid" ? node.specifiers : [];
                for (const specifier of imported) {
                    const generator =
                        specifier.type === "ImportSpecifier" &&
                        specifier.imported.type === "Identifier"
                            ? specifier.imported.name
                            : "";
                    const uuid = UUIDS.get(generator);
                    if (uuid !== undefined) {
                        const data = { name: `uuid's ${generator}`, replacement: uuid };
                        context.report({ node: specifier, messageId: "effect", data });
                    }
                }
            },
            MemberExpression(node) {
                // read a host global's member by any spelling
                const { object } = node;
                const member = memberName(node);
                if (object.type !== "Identifier" || member === undefined) {
                    return;
                }

                // report a named member, any member of a host global, and a host global through globalThis
                const name = `${object.name}.${member}`;
                const isGlobal = GLOBAL_OBJECTS.has(object.name);
                const replacement =
                    MEMBERS.get(name) ??
                    HOSTS.get(isGlobal ? member : object.name) ??
                    (isGlobal ? FUNCTIONS.get(member) : undefined);
                if (
                    replacement !== undefined &&
                    !PURE_MEMBERS.has(name) &&
                    !isLocal(node, object.name)
                ) {
                    context.report({ node, messageId: "effect", data: { name, replacement } });
                }
            },
            VariableDeclarator(node) {
                // read the members a pattern takes from a host global
                const { id, init } = node;
                if (
                    id.type !== "ObjectPattern" ||
                    init?.type !== "Identifier" ||
                    isLocal(node, init.name)
                ) {
                    return;
                }

                // report each member taken
                for (const property of id.properties) {
                    const key =
                        property.type === "Property" && property.key.type === "Identifier"
                            ? property.key.name
                            : "";
                    const name = `${init.name}.${key}`;
                    const replacement = MEMBERS.get(name) ?? HOSTS.get(init.name);
                    if (key !== "" && replacement !== undefined && !PURE_MEMBERS.has(name)) {
                        context.report({
                            node: property,
                            messageId: "effect",
                            data: { name, replacement },
                        });
                    }
                }
            },
            AssignmentExpression(node) {
                // report replacing the host's console
                const { left } = node;
                const isConsole =
                    left.type === "MemberExpression" &&
                    left.object.type === "Identifier" &&
                    GLOBAL_OBJECTS.has(left.object.name) &&
                    memberName(left) === "console";
                if (isConsole) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: {
                            name: "globalThis.console",
                            replacement: "environment.process.stderr",
                        },
                    });
                }
            },
            BinaryExpression(node) {
                // report probing globalThis for a host global
                const { left, right } = node;
                const probed =
                    left.type === "Literal" && typeof left.value === "string" ? left.value : "";
                const replacement = HOSTS.get(probed);
                const isProbe =
                    node.operator === "in" &&
                    right.type === "Identifier" &&
                    GLOBAL_OBJECTS.has(right.name);
                if (isProbe && replacement !== undefined) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: { name: `globalThis.${probed}`, replacement },
                    });
                }
            },
            CallExpression(node) {
                // report a host function, and formatting in the host's locale
                const { callee } = node;
                const name = callee.type === "Identifier" ? callee.name : "";
                const replacement = isLocal(node, name) ? undefined : FUNCTIONS.get(name);
                const method =
                    callee.type === "MemberExpression" && callee.property.type === "Identifier"
                        ? callee.property.name
                        : "";
                // report a host function
                if (replacement !== undefined) {
                    context.report({ node, messageId: "effect", data: { name, replacement } });
                }
                // report formatting in the host's locale
                else if (FORMATTING.has(method) && node.arguments.length === 0) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: { name: `${method}()`, replacement: "environment.locale" },
                    });
                }
            },
            NewExpression(node) {
                // report a host constructor, and an Intl formatter in the host's locale
                const { callee } = node;
                const constructed = callee.type === "Identifier" ? callee.name : "";
                const replacement = isLocal(node, constructed)
                    ? undefined
                    : CONSTRUCTORS.get(constructed);
                const isIntl =
                    callee.type === "MemberExpression" &&
                    callee.object.type === "Identifier" &&
                    callee.object.name === "Intl" &&
                    node.arguments.length === 0;
                // report a host constructor
                if (replacement !== undefined) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: { name: `new ${constructed}`, replacement },
                    });
                }
                // report an Intl formatter in the host's locale
                else if (isIntl) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: { name: "Intl without a locale", replacement: "environment.locale" },
                    });
                }

                // report a date read from the host's clock
                const isClock =
                    node.callee.type === "Identifier" &&
                    node.callee.name === "Date" &&
                    node.arguments.length === 0;
                if (isClock) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: {
                            name: "new Date()",
                            replacement: "new Date(environment.clock.now())",
                        },
                    });
                }
            },
        };
    },
};

/** Read a member expression's member name, written plainly or as a string, absent otherwise. */
function memberName(node: ESTree.MemberExpression): string | undefined {
    const { property } = node;

    // read a plain member
    if (!node.computed && property.type === "Identifier") {
        return property.name;
    }
    // read a member written as a string
    else if (node.computed && property.type === "Literal" && typeof property.value === "string") {
        return property.value;
    }
    // leave a computed member
    else {
        return undefined;
    }
}
