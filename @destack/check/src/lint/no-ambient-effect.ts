import type { ESTree, Rule } from "@oxlint/plugins";
import { isEnvironmentFile } from "./package.ts";

/** The host modules each effect comes from, and the environment member taking their place. */
const MODULES = new Map([
    ["node:fs", "environment.files"],
    ["node:fs/promises", "environment.files"],
    ["node:child_process", "environment.process.spawn"],
    ["node:os", "environment.host"],
    ["fs", "environment.files"],
    ["fs/promises", "environment.files"],
    ["child_process", "environment.process.spawn"],
    ["os", "environment.host"],
    ["node:net", "environment.network.connect"],
    ["node:http", "environment.server.http"],
    ["node:https", "environment.network.fetch"],
    ["node:tls", "environment.network.connect"],
    ["node:dgram", "environment.network"],
    ["node:worker_threads", "environment.workers"],
    ["net", "environment.network.connect"],
    ["http", "environment.server.http"],
    ["https", "environment.network.fetch"],
    ["tls", "environment.network.connect"],
    ["dgram", "environment.network"],
    ["worker_threads", "environment.workers"],
]);

/** The crypto modules, whose randomness and key generation are effects and whose hashing is not. */
const CRYPTO_MODULES = new Set(["node:crypto", "crypto"]);

/** The crypto modules' exports that compute from their inputs alone, such as hashes. */
const PURE_CRYPTO = new Set([
    "createHash",
    "createHmac",
    "createCipheriv",
    "createDecipheriv",
    "hash",
    "timingSafeEqual",
    "createPublicKey",
    "createPrivateKey",
    "createSecretKey",
    "createVerify",
    "verify",
    "hkdfSync",
    "pbkdf2Sync",
    "scryptSync",
    "Hash",
    "Hmac",
    "KeyObject",
]);

/** The host globals whose every member is an effect, and the environment taking their place. */
const HOSTS = new Map([
    ["process", "environment.process"],
    ["Bun", "the environment"],
    ["localStorage", "environment.storage"],
    ["sessionStorage", "environment.storage"],
    ["location", "environment.navigation.currentEntry"],
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
    ["process.platform", "environment.host.os"],
    ["process.arch", "environment.host.architecture"],
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
    ["navigator.locks", "environment.locks"],
    ["navigator.sendBeacon", "environment.network.fetch"],
    ["navigator.serviceWorker", "environment.workers"],
    ["navigator.storage", "environment.storage"],
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
    ["setTimeout", "environment.clock.setTimeout"],
    ["setInterval", "environment.clock.setInterval"],
    ["fetch", "environment.network.fetch"],
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

        /** Report whether the file declares a name or imports it from the workspace. */
        function isLocal(node: ESTree.Node, name: string): boolean {
            // walk the scopes outward to the name's declaration
            let scope: ReturnType<typeof context.sourceCode.getScope> | null =
                context.sourceCode.getScope(node);
            while (scope) {
                const definition = scope.set.get(name)?.defs[0];
                if (definition?.type === "ImportBinding") {
                    return isLocalSource(definition.parent);
                } else if (definition !== undefined) {
                    return true;
                }
                scope = scope.upper;
            }

            return false;
        }

        /** Report a host module a declaration takes, and each effect among the exports it names. */
        function reportSource(
            node: ESTree.Node,
            name: string,
            names: readonly (string | undefined)[],
        ): void {
            // report a host module
            const replacement = MODULES.get(name);
            if (replacement !== undefined) {
                context.report({ node, messageId: "effect", data: { name, replacement } });
            }

            // report each crypto export that draws randomness or makes keys, and each uuid generator
            for (const exported of names) {
                const isCryptoEffect =
                    CRYPTO_MODULES.has(name) &&
                    exported !== undefined &&
                    !PURE_CRYPTO.has(exported);
                const uuid =
                    name === "uuid" && exported !== undefined ? UUIDS.get(exported) : undefined;
                if (isCryptoEffect) {
                    const data = {
                        name: exported === "*" ? `all of ${name}` : `${name}'s ${exported}`,
                        replacement: "environment.random or environment.crypto",
                    };
                    context.report({ node, messageId: "effect", data });
                } else if (uuid !== undefined) {
                    context.report({
                        node,
                        messageId: "effect",
                        data: { name: `uuid's ${exported}`, replacement: uuid },
                    });
                }
            }
        }

        return {
            ImportDeclaration(node) {
                // report a host module, each effect it names, and each generator of the uuid package
                const isType = node.importKind === "type";
                const names = node.specifiers.map((specifier) =>
                    specifier.type === "ImportSpecifier" && specifier.importKind !== "type"
                        ? importedName(specifier.imported)
                        : specifier.type === "ImportSpecifier"
                          ? undefined
                          : "*",
                );
                reportSource(node, node.source.value, isType ? [] : names);
            },
            ExportNamedDeclaration(node) {
                // report a host module re-exported, and each effect it passes on
                if (node.source !== null && node.exportKind !== "type") {
                    const names = node.specifiers.map((specifier) => importedName(specifier.local));
                    reportSource(node, node.source.value, names);
                }
            },
            ExportAllDeclaration(node) {
                // report a host module re-exported whole
                if (node.exportKind !== "type") {
                    reportSource(node, node.source.value, ["*"]);
                }
            },
            ImportExpression(node) {
                // report a host module imported at run time, whose members go unseen
                if (node.source.type === "Literal" && typeof node.source.value === "string") {
                    reportSource(node, node.source.value, ["*"]);
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

/** Read the name an import or export specifier refers to in its module. */
function importedName(name: ESTree.ModuleExportName): string {
    return name.type === "Identifier" ? name.name : name.value;
}

/** Report whether an import takes from the workspace or a relative path. */
function isLocalSource(declaration: ESTree.Node | null | undefined): boolean {
    if (declaration?.type !== "ImportDeclaration") {
        return false;
    }
    const source = declaration.source.value;

    return source.startsWith(".") || source.startsWith("@destack/");
}
