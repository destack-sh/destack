import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

import {
    FALLBACK_COMMANDS,
    LIVE_SETTING_KEYS,
    SERVER_SETTING_KEYS,
    WORKSPACE_BINARY_CANDIDATES,
} from "./constants";

/**
 * The resolved launch shape for the Destack language server process.
 */
export type ResolvedServerCommand = {
    /**
     * The resolved command path.
     */
    command: string;
    /**
     * The command arguments.
     */
    args: string[];
    /**
     * The optional working directory.
     */
    cwd?: string;
};

/**
 * Return the best active workspace folder for command resolution.
 */
export function getActiveWorkspaceFolder(): vscode.WorkspaceFolder | undefined {
    // prefer the workspace folder for the active editor
    const activeDocumentUri = vscode.window.activeTextEditor?.document.uri;
    if (activeDocumentUri) {
        const activeFolder = vscode.workspace.getWorkspaceFolder(activeDocumentUri);
        if (activeFolder) {
            return activeFolder;
        }
    }

    // otherwise fall back to the first open folder
    return vscode.workspace.workspaceFolders?.[0];
}

/**
 * Return true when configuration changes require a process restart.
 */
export function isServerConfigurationChange(event: vscode.ConfigurationChangeEvent): boolean {
    return SERVER_SETTING_KEYS.some((settingKey) => event.affectsConfiguration(settingKey));
}

/**
 * Return true when configuration changes can be pushed live.
 */
export function isLiveConfigurationChange(event: vscode.ConfigurationChangeEvent): boolean {
    return LIVE_SETTING_KEYS.some((settingKey) => event.affectsConfiguration(settingKey));
}

/**
 * Resolve the final language server command from settings and environment.
 */
export function resolveServerCommand(
    configuration: vscode.WorkspaceConfiguration,
    workspaceFolder: vscode.WorkspaceFolder | undefined,
): ResolvedServerCommand {
    // read configured launch settings
    const rawCommand = configuration.get<string>("server.command") ?? "";
    const rawArguments = configuration.get<string[]>("server.args") ?? [];
    const rawWorkingDirectory = configuration.get<string>("server.cwd") ?? "";

    // resolve workspace context
    const workspaceRoot = workspaceFolder?.uri.fsPath;
    const workingDirectory = rawWorkingDirectory
        ? expandPath(rawWorkingDirectory, workspaceFolder)
        : workspaceRoot;

    // resolve explicit command setting first
    if (rawCommand) {
        return resolveExplicitCommand(rawCommand, rawArguments, workspaceFolder, workspaceRoot, workingDirectory);
    }

    // resolve workspace built binary next
    if (workspaceRoot) {
        const workspaceBinary = resolveWorkspaceBinary(workspaceRoot);
        if (workspaceBinary) {
            return {
                command: workspaceBinary,
                args: withLspSubcommand(rawArguments),
                cwd: workspaceRoot,
            };
        }
    }

    // resolve fallback commands on path last
    for (const fallbackCommand of FALLBACK_COMMANDS) {
        const resolvedFallbackCommand = resolveCommandOnPath(fallbackCommand);
        if (resolvedFallbackCommand) {
            return {
                command: resolvedFallbackCommand,
                args: withLspSubcommand(rawArguments),
                cwd: workingDirectory,
            };
        }
    }

    // fail loudly with full resolution context
    const resolutionHint = binaryResolutionHint(workspaceRoot);
    throw new Error(
        `destack.server.command is unset and no Destack binary was found; ${resolutionHint}.`,
    );
}

/**
 * Resolve and validate an explicit command setting.
 */
function resolveExplicitCommand(
    rawCommand: string,
    rawArguments: string[],
    workspaceFolder: vscode.WorkspaceFolder | undefined,
    workspaceRoot: string | undefined,
    workingDirectory: string | undefined,
): ResolvedServerCommand {
    // expand workspace placeholders and home directory
    const expandedCommand = expandPath(rawCommand, workspaceFolder);

    // resolve command on path when configured as bare executable name
    const resolvedCommand = resolveCommandPath(expandedCommand);

    // validate explicit absolute or relative command paths
    const isPathLikeCommand =
        path.isAbsolute(expandedCommand) || expandedCommand.includes(path.sep);
    if (!resolvedCommand && isPathLikeCommand) {
        const resolutionHint = binaryResolutionHint(workspaceRoot);
        throw new Error(`destack.server.command not found: ${expandedCommand}; ${resolutionHint}`);
    }

    // compute final command and argument list
    const command = resolvedCommand ?? expandedCommand;
    const args = shouldInjectLspSubcommand(command, rawArguments)
        ? withLspSubcommand(rawArguments)
        : rawArguments;

    return { command, args, cwd: workingDirectory };
}

/**
 * Expand workspace and home-directory prefixes.
 */
function expandPath(value: string, workspaceFolder?: vscode.WorkspaceFolder): string {
    if (!value) {
        return value;
    }

    // expand vscode workspace token
    let expanded = value;
    if (workspaceFolder) {
        expanded = expanded.replace(/\$\{workspaceFolder\}/g, workspaceFolder.uri.fsPath);
    }

    // expand home shorthand
    if (expanded == "~") {
        return os.homedir();
    }

    // expand home shorthand prefix
    if (expanded.startsWith(`~${path.sep}`)) {
        return path.join(os.homedir(), expanded.slice(2));
    }

    return expanded;
}

/**
 * Resolve a command path from PATH.
 */
function resolveCommandOnPath(command: string): string | undefined {
    // derive path search roots and executable extensions
    const pathEnvironment = process.env.PATH || "";
    const executableExtensions =
        process.platform == "win32"
            ? (process.env.PATHEXT || ".EXE;.CMD;.BAT;.COM").split(";")
            : [""];

    // search each directory with each platform extension
    for (const searchDirectory of pathEnvironment.split(path.delimiter)) {
        if (!searchDirectory) {
            continue;
        }

        for (const executableExtension of executableExtensions) {
            const candidateCommand = path.join(
                searchDirectory,
                `${command}${executableExtension}`,
            );
            if (fs.existsSync(candidateCommand)) {
                return candidateCommand;
            }
        }
    }

    return undefined;
}

/**
 * Resolve a command from path-like and bare command forms.
 */
function resolveCommandPath(command: string): string | undefined {
    if (!command) {
        return undefined;
    }

    // keep explicit path-like commands as-is
    const isPathLikeCommand = path.isAbsolute(command) || command.includes(path.sep);
    if (isPathLikeCommand) {
        return command;
    }

    // resolve bare command names from path
    return resolveCommandOnPath(command);
}

/**
 * Ensure the `lsp` subcommand is present.
 */
function withLspSubcommand(args: string[]): string[] {
    if (args[0] == "lsp") {
        return args;
    }

    return ["lsp", ...args];
}

/**
 * Resolve workspace-local binary candidates.
 */
function resolveWorkspaceBinary(workspaceRoot: string): string | undefined {
    for (const relativeCandidatePath of WORKSPACE_BINARY_CANDIDATES) {
        const candidateCommand = path.join(workspaceRoot, relativeCandidatePath);
        if (fs.existsSync(candidateCommand)) {
            return candidateCommand;
        }
    }

    return undefined;
}

/**
 * Build a diagnostic resolution hint for startup failures.
 */
function binaryResolutionHint(workspaceRoot: string | undefined): string {
    // list workspace binary candidates
    const workspaceCandidates = workspaceRoot
        ? WORKSPACE_BINARY_CANDIDATES.map((relativePath) => path.join(workspaceRoot, relativePath))
        : [];
    const workspaceSummary =
        workspaceCandidates.length > 0 ? workspaceCandidates.join(", ") : "(no workspace folder)";

    // list fallback commands
    const fallbackSummary = FALLBACK_COMMANDS.join(", ");

    return `checked workspace binaries [${workspaceSummary}] and PATH commands (${fallbackSummary})`;
}

/**
 * Return true when known Destack wrapper binaries need `lsp` injection.
 */
function shouldInjectLspSubcommand(command: string, args: string[]): boolean {
    if (args.length > 0 && args[0] == "lsp") {
        return false;
    }

    const commandBaseName = path.basename(command).toLowerCase();
    const normalizedCommand = commandBaseName.endsWith(".exe")
        ? commandBaseName.slice(0, -4)
        : commandBaseName;

    return FALLBACK_COMMANDS.includes(normalizedCommand);
}
