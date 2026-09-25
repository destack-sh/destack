import { createHash } from "node:crypto";
import { constants as fileConstants } from "node:fs";
import { access, chmod, mkdir, mkdtemp, rename, rm, stat, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import * as path from "node:path";
import extractZip from "extract-zip";
import * as tar from "tar";
import * as vscode from "vscode";
import type { Executable } from "vscode-languageclient/node";

const COMMAND_NAME = "tspp";
const RELEASE_REPOSITORY = "destack-sh/tspp";
const DOWNLOAD_TIMEOUT_MILLISECONDS = 60_000;

/** One supported TS++ release target. */
type ReleaseTarget = {
    /** The Rust target triple. */
    triple: string;
    /** The release archive extension. */
    archiveExtension: "tar.gz" | "zip";
    /** The platform executable name. */
    executableName: string;
};

/** The resolved TS++ language server command. */
export class ServerCommand {
    /** Executable command. */
    readonly command: string;
    /** Command arguments. */
    readonly arguments_: string[];
    /** Process working directory. */
    readonly workingDirectory: string | undefined;

    /** Create one resolved command. */
    private constructor(
        command: string,
        arguments_: string[],
        workingDirectory: string | undefined,
    ) {
        this.command = command;
        this.arguments_ = arguments_;
        this.workingDirectory = workingDirectory;
    }

    /** Resolve the language server command for one extension session. */
    static async resolve(context: vscode.ExtensionContext): Promise<ServerCommand> {
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        const configuration = vscode.workspace.getConfiguration("tspp", workspaceFolder?.uri);
        const workingDirectory = this.workingDirectory(configuration, workspaceFolder);
        const configuredCommand = configuration.get<string>("server.command")?.trim();
        const arguments_ = configuration.get<string[]>("server.args") ?? [];

        // use the configured command as the authoritative selection
        if (configuredCommand) {
            const command = await this.configured(
                configuredCommand,
                arguments_,
                workspaceFolder,
                workingDirectory,
            );

            return command;
        }

        // prefer repository builds during toolchain development
        const workspaceCommand = await this.workspace(workspaceFolder, arguments_);
        if (workspaceCommand) {
            return workspaceCommand;
        }

        // use an installed toolchain when available
        const installedCommand = await this.installed(workingDirectory, arguments_);
        if (installedCommand) {
            return installedCommand;
        }

        // install the matching release for a clean editor installation
        const managedCommand = await this.managed(context, workingDirectory, arguments_);

        return managedCommand;
    }

    /** Build native language client executable options. */
    executable(): Executable {
        return {
            command: this.command,
            args: this.arguments_,
            options: {
                cwd: this.workingDirectory,
                shell: false,
            },
        };
    }

    /** Resolve one explicitly configured command. */
    private static async configured(
        configuredCommand: string,
        configuredArguments: string[],
        workspaceFolder: vscode.WorkspaceFolder | undefined,
        workingDirectory: string | undefined,
    ): Promise<ServerCommand> {
        const expandedCommand = this.expand(configuredCommand, workspaceFolder);
        const command = this.isPath(expandedCommand)
            ? this.resolvePath(expandedCommand, workingDirectory)
            : await this.commandOnPath(expandedCommand);
        if (!command || !(await this.isExecutable(command))) {
            throw new Error(`configured TS++ command is not executable: ${expandedCommand}`);
        }

        const arguments_ = this.withLspSubcommand(command, configuredArguments);

        return new ServerCommand(command, arguments_, workingDirectory);
    }

    /** Resolve the first workspace-built toolchain command. */
    private static async workspace(
        workspaceFolder: vscode.WorkspaceFolder | undefined,
        arguments_: string[],
    ): Promise<ServerCommand | undefined> {
        if (!workspaceFolder || !vscode.workspace.isTrusted) {
            return undefined;
        }

        const root = workspaceFolder.uri.fsPath;
        const executableName = process.platform === "win32" ? "tspp.exe" : COMMAND_NAME;
        const candidates = [
            path.join(root, "target", "release", executableName),
            path.join(root, "target", "debug", executableName),
        ];

        // select the first executable candidate in build-profile order
        for (const candidate of candidates) {
            if (await this.isExecutable(candidate)) {
                const argumentsWithSubcommand = this.withLspSubcommand(candidate, arguments_);

                return new ServerCommand(candidate, argumentsWithSubcommand, root);
            }
        }

        return undefined;
    }

    /** Resolve one installed toolchain command. */
    private static async installed(
        workingDirectory: string | undefined,
        arguments_: string[],
    ): Promise<ServerCommand | undefined> {
        const command = await this.commandOnPath(COMMAND_NAME);
        if (!command) {
            return undefined;
        }
        const argumentsWithSubcommand = this.withLspSubcommand(command, arguments_);

        return new ServerCommand(command, argumentsWithSubcommand, workingDirectory);
    }

    /** Resolve or install the toolchain release matching this extension. */
    private static async managed(
        context: vscode.ExtensionContext,
        workingDirectory: string | undefined,
        arguments_: string[],
    ): Promise<ServerCommand> {
        const version = context.extension.packageJSON.version;
        if (typeof version !== "string" || version.length === 0) {
            throw new Error("TS++ extension manifest has no version");
        }

        const target = this.releaseTarget();
        const releaseDirectory = vscode.Uri.joinPath(
            context.globalStorageUri,
            `tspp-${version}-${target.triple}`,
        ).fsPath;
        const command = path.join(releaseDirectory, target.executableName);
        if (!(await this.isExecutable(command))) {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Installing TS++ ${version}`,
                    cancellable: false,
                },
                async () => this.install(version, target, releaseDirectory),
            );
        }
        if (!(await this.isExecutable(command))) {
            throw new Error(`installed TS++ release has no executable: ${command}`);
        }
        const argumentsWithSubcommand = this.withLspSubcommand(command, arguments_);

        return new ServerCommand(command, argumentsWithSubcommand, workingDirectory);
    }

    /** Install one verified TS++ release archive. */
    private static async install(
        version: string,
        target: ReleaseTarget,
        releaseDirectory: string,
    ): Promise<void> {
        const archiveName = `tspp-${version}-${target.triple}.${target.archiveExtension}`;
        const releaseUrl = `https://github.com/${RELEASE_REPOSITORY}/releases/download/v${version}`;
        const parentDirectory = path.dirname(releaseDirectory);
        const command = path.join(releaseDirectory, target.executableName);

        // allocate one isolated installation directory
        await mkdir(parentDirectory, { recursive: true });
        const temporaryDirectory = await mkdtemp(
            path.join(parentDirectory, `.tspp-${version}-${target.triple}-`),
        );
        const archivePath = path.join(temporaryDirectory, archiveName);
        const extractedDirectory = path.join(temporaryDirectory, "extracted");

        // build the complete installation in an isolated staging directory
        await mkdir(extractedDirectory, { recursive: true });
        try {
            const [archive, checksums] = await Promise.all([
                this.download(`${releaseUrl}/${archiveName}`),
                this.download(`${releaseUrl}/SHA256SUMS`),
            ]);
            this.verifyArchive(archiveName, archive, checksums.toString("utf8"));
            await writeFile(archivePath, archive);

            // extract the verified platform archive
            if (target.archiveExtension === "zip") {
                await extractZip(archivePath, { dir: extractedDirectory });
            } else {
                await tar.extract({
                    cwd: extractedDirectory,
                    file: archivePath,
                    preservePaths: false,
                    strict: true,
                });
            }

            // publish the release directory atomically
            const packageDirectory = path.join(
                extractedDirectory,
                `tspp-${version}-${target.triple}`,
            );
            const executable = path.join(packageDirectory, target.executableName);
            if (!(await this.isFile(executable))) {
                throw new Error(`TS++ release archive is missing ${target.executableName}`);
            }
            if (process.platform !== "win32") {
                await chmod(executable, 0o755);
            }

            // publish atomically or retain a concurrent completed installation
            try {
                await rename(packageDirectory, releaseDirectory);
            } catch (error) {
                if (!(await this.isExecutable(command))) {
                    throw error;
                }
            }
        } finally {
            await rm(temporaryDirectory, { recursive: true, force: true });
        }
    }

    /** Download one bounded release resource. */
    private static async download(url: string): Promise<Buffer> {
        const response = await fetch(url, {
            headers: { "user-agent": "tspp-vscode" },
            signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MILLISECONDS),
        });
        if (!response.ok) {
            throw new Error(`failed to download ${url}: HTTP ${response.status}`);
        }

        return Buffer.from(await response.arrayBuffer());
    }

    /** Verify one release archive against its published SHA-256 digest. */
    private static verifyArchive(archiveName: string, archive: Buffer, checksums: string): void {
        const checksumLine = checksums
            .split(/\r?\n/)
            .find((line) => line.trimEnd().endsWith(` ${archiveName}`));
        const expected = checksumLine?.trim().split(/\s+/)[0]?.toLowerCase();
        if (!expected || !/^[0-9a-f]{64}$/.test(expected)) {
            throw new Error(`TS++ release has no valid checksum for ${archiveName}`);
        }

        const actual = createHash("sha256").update(archive).digest("hex");
        if (actual !== expected) {
            throw new Error(`TS++ release checksum differs for ${archiveName}`);
        }
    }

    /** Resolve the active release target. */
    private static releaseTarget(): ReleaseTarget {
        const platform = process.platform;
        const architecture = process.arch;
        if (platform === "darwin" && architecture === "arm64") {
            return {
                triple: "aarch64-apple-darwin",
                archiveExtension: "tar.gz",
                executableName: COMMAND_NAME,
            };
        }
        if (platform === "darwin" && architecture === "x64") {
            return {
                triple: "x86_64-apple-darwin",
                archiveExtension: "tar.gz",
                executableName: COMMAND_NAME,
            };
        }
        if (platform === "linux" && architecture === "arm64") {
            return {
                triple: "aarch64-unknown-linux-gnu",
                archiveExtension: "tar.gz",
                executableName: COMMAND_NAME,
            };
        }
        if (platform === "linux" && architecture === "x64") {
            return {
                triple: "x86_64-unknown-linux-gnu",
                archiveExtension: "tar.gz",
                executableName: COMMAND_NAME,
            };
        }
        if (platform === "win32" && architecture === "x64") {
            return {
                triple: "x86_64-pc-windows-msvc",
                archiveExtension: "zip",
                executableName: "tspp.exe",
            };
        }

        throw new Error(`TS++ has no release for ${platform}/${architecture}`);
    }

    /** Resolve the configured working directory. */
    private static workingDirectory(
        configuration: vscode.WorkspaceConfiguration,
        workspaceFolder: vscode.WorkspaceFolder | undefined,
    ): string | undefined {
        const configured = configuration.get<string>("server.cwd")?.trim();
        if (configured) {
            const expanded = this.expand(configured, workspaceFolder);

            return this.resolvePath(expanded, workspaceFolder?.uri.fsPath);
        }

        return workspaceFolder?.uri.fsPath;
    }

    /** Expand supported editor path variables. */
    private static expand(
        value: string,
        workspaceFolder: vscode.WorkspaceFolder | undefined,
    ): string {
        let expanded = value;
        if (workspaceFolder) {
            expanded = expanded.replaceAll("${workspaceFolder}", workspaceFolder.uri.fsPath);
        }
        if (expanded === "~") {
            return homedir();
        }
        if (expanded.startsWith(`~${path.sep}`)) {
            return path.join(homedir(), expanded.slice(2));
        }

        return expanded;
    }

    /** Resolve one explicit path against its configured directory. */
    private static resolvePath(value: string, directory: string | undefined): string {
        if (path.isAbsolute(value)) {
            return value;
        }
        if (!directory) {
            throw new Error(`relative TS++ path requires a workspace or server.cwd: ${value}`);
        }

        return path.resolve(directory, value);
    }

    /** Resolve one executable from the process search path. */
    private static async commandOnPath(command: string): Promise<string | undefined> {
        const searchPath = process.env.PATH ?? "";
        const extensions =
            process.platform === "win32"
                ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT;.COM").split(";")
                : [""];

        // search each configured directory in process order
        for (const directory of searchPath.split(path.delimiter)) {
            if (!directory) {
                continue;
            }
            for (const extension of extensions) {
                const candidate = path.join(directory, `${command}${extension}`);
                if (await this.isExecutable(candidate)) {
                    return candidate;
                }
            }
        }

        return undefined;
    }

    /** Return command arguments with the TS++ LSP subcommand when required. */
    private static withLspSubcommand(command: string, arguments_: string[]): string[] {
        const executableName = path
            .basename(command)
            .toLowerCase()
            .replace(/\.exe$/, "");
        if (executableName === COMMAND_NAME && arguments_[0] !== "lsp") {
            return ["lsp", ...arguments_];
        }

        return arguments_;
    }

    /** Return whether one value names an explicit path. */
    private static isPath(value: string): boolean {
        return path.isAbsolute(value) || value.includes("/") || value.includes("\\");
    }

    /** Return whether one path is an executable file. */
    private static async isExecutable(filePath: string): Promise<boolean> {
        const mode = process.platform === "win32" ? fileConstants.F_OK : fileConstants.X_OK;
        try {
            // require a nonempty regular file
            const file = await stat(filePath);
            if (!file.isFile() || file.size === 0) {
                return false;
            }

            // require platform execute access
            await access(filePath, mode);

            return true;
        } catch {
            return false;
        }
    }

    /** Return whether one path is a regular file. */
    private static async isFile(filePath: string): Promise<boolean> {
        try {
            const file = await stat(filePath);

            return file.isFile() && file.size > 0;
        } catch {
            return false;
        }
    }
}
