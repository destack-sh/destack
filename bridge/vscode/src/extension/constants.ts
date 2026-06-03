import * as path from "node:path";

/** Fallback command names used when no explicit binary is configured. */
export const FALLBACK_COMMANDS = ["destack", "ds", "dsc"];

/** Workspace relative binary candidates checked before PATH fallback. */
export const WORKSPACE_BINARY_CANDIDATES =
    process.platform == "win32"
        ? [
              path.join("target", "release", "destack.exe"),
              path.join("target", "debug", "destack.exe"),
          ]
        : [path.join("target", "release", "destack"), path.join("target", "debug", "destack")];

/** Settings that require a full lsp restart when changed. */
export const SERVER_SETTING_KEYS = [
    "destack.server.command",
    "destack.server.args",
    "destack.server.cwd",
];

/** Settings that can be pushed live without restart. */
export const LIVE_SETTING_KEYS = [
    "destack.completion.autoImports",
    "destack.inlayHints.parameterHints",
    "destack.inlayHints.typeHints",
];
