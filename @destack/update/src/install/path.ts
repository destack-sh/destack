import { basename, dirname, join, relative } from "node:path";
import { FileSystemError } from "@destack/environment";
import type { DistributionEnvironment } from "../distribution/distribution.ts";
import type { Stage } from "../release/index.ts";
import { UpdateError } from "../error/error.ts";

/** How long PowerShell may take to change the user's PATH, in milliseconds: a start and a write. */
const POWERSHELL_TIMEOUT_MILLISECONDS = 10_000;

/** The permissions of a startup file the installer creates: the person's alone. */
const STARTUP_FILE_MODE = 0o600;

/** The PATH entry of a stage's commands in the user's shell or Windows environment. */
export class ShellPath {
    /** The directory with the stage's commands, under `$HOME` when inside the home directory. */
    readonly bin: string;
    /** The stage whose commands the entry adds. */
    readonly stage: Stage;
    /** The host whose shell files and Windows environment hold the entry. */
    readonly host: DistributionEnvironment;

    /** Write the entry of a stage's command directory on a host. */
    constructor(bin: string, stage: Stage, host: DistributionEnvironment) {
        this.bin = bin;
        this.stage = stage;
        this.host = host;
    }

    /** Register the stage's terminal command in the user's shell. */
    async add(): Promise<void> {
        // let Windows retain the user PATH independently of the current process
        const { home } = this.host.host;
        const { variables } = this.host.process;
        const shell = basename(variables["SHELL"] ?? "sh");
        if (this.host.host.os === "win32") {
            await this.#powershell(
                "PATH addition",
                "$path = [Environment]::GetEnvironmentVariable('Path', 'User'); " +
                    "if (($path -split ';') -notcontains $env:DESTACK_BIN) { " +
                    "[Environment]::SetEnvironmentVariable('Path', ($env:DESTACK_BIN + ';' + $path), 'User') }",
            );
        }
        // add fish's own configuration file
        else if (shell === "fish") {
            await this.host.files.mkdir(dirname(this.#fishFile), { recursive: true });
            await this.#append(this.#fishFile, this.#fish);
        }
        // add to zsh's startup file
        else if (shell === "zsh") {
            await this.#append(join(variables["ZDOTDIR"] ?? home, ".zshrc"), this.#path);
        }
        // add to bash's interactive file and its first login file
        else if (shell === "bash") {
            await this.#append(join(home, ".bashrc"), this.#path);
            const logins = [".bash_profile", ".bash_login", ".profile"].map((name) =>
                join(home, name),
            );
            const found = await Promise.all(logins.map((path) => this.host.files.exists(path)));
            await this.#append(logins[found.indexOf(true)] ?? join(home, ".profile"), this.#path);
        }
        // add to the POSIX shell's profile
        else if (shell === "sh") {
            await this.#append(join(home, ".profile"), this.#path);
        }
        // ask the person to add it in another shell
        else {
            await this.host.process.stderr.write(
                `Add ${this.bin} to PATH in your ${shell} configuration.\n`,
            );
        }
    }

    /** Remove only the exact PATH entries the stage's installer wrote. */
    async remove(): Promise<void> {
        // remove this exact directory from the Windows user's PATH
        const { home } = this.host.host;
        const { variables } = this.host.process;
        if (this.host.host.os === "win32") {
            await this.#powershell(
                "PATH removal",
                "$path = [Environment]::GetEnvironmentVariable('Path', 'User'); " +
                    "$entries = @($path -split ';' | Where-Object { $_ -ne $env:DESTACK_BIN }); " +
                    "[Environment]::SetEnvironmentVariable('Path', ($entries -join ';'), 'User')",
            );
        }
        // remove the registration's lines from each known shell file that has them
        else {
            const paths = new Set([
                ...[".zshrc", ".bashrc", ".bash_profile", ".bash_login", ".profile"].map((file) =>
                    join(home, file),
                ),
                join(variables["ZDOTDIR"] ?? home, ".zshrc"),
                this.#fishFile,
            ]);
            for (const path of paths) {
                const source = await this.#read(path);
                const next = source
                    ?.replaceAll(`\n# Destack\n${this.#path}\n`, "")
                    .replaceAll(`\n# Destack\n${this.#fish}\n`, "");
                if (source !== undefined && next !== source) {
                    await this.host.files.writeTextFile(path, next ?? "");
                }
            }
        }
    }

    /** The directory as shell files write it, under `$HOME` when inside the home directory. */
    get #written(): string {
        const inside = relative(this.host.host.home, this.bin);

        return inside.startsWith("..") ? this.bin : `$HOME/${inside}`;
    }

    /** The POSIX shell line adding the stage's commands to PATH once. */
    get #path(): string {
        return `case ":$PATH:" in *":${this.#written}:"*) ;; *) export PATH="${this.#written}:$PATH" ;; esac`;
    }

    /** The fish line adding the stage's commands to PATH. */
    get #fish(): string {
        return `fish_add_path --path "${this.#written}"`;
    }

    /** The fish configuration file with the stage's line. */
    get #fishFile(): string {
        const { variables } = this.host.process;
        const configuration = variables["XDG_CONFIG_HOME"] ?? join(this.host.host.home, ".config");

        return join(configuration, "fish/conf.d", `${this.stage.command}.fish`);
    }

    /** Append one registration unless the file already contains it. */
    async #append(path: string, line: string): Promise<void> {
        const source = (await this.#read(path)) ?? "";
        if (!source.split("\n").includes(line)) {
            await this.host.files.writeTextFile(path, `\n# Destack\n${line}\n`, {
                append: true,
                mode: STARTUP_FILE_MODE,
            });
        }
    }

    /** Read a startup file's text, absent for a missing file. */
    async #read(path: string): Promise<string | undefined> {
        return this.host.files.readTextFile(path).catch((error: unknown) => {
            if (!FileSystemError.is(error, "NOT_FOUND")) {
                throw error;
            }

            return undefined;
        });
    }

    /** Run a PowerShell script on the user's environment with the directory as DESTACK_BIN. */
    async #powershell(action: string, script: string): Promise<void> {
        // stop at the script's first error
        const { process, clock } = this.host;
        const child = process.spawn(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                `$ErrorActionPreference = 'Stop'; ${script}`,
            ],
            {
                variables: { ...process.variables, DESTACK_BIN: this.bin },
                stdout: "ignore",
                stderr: "pipe",
                signal: clock.timeout(POWERSHELL_TIMEOUT_MILLISECONDS),
            },
        );

        // report the failure with what PowerShell wrote
        const [code, error] = await Promise.all([child.exited, new Response(child.stderr).text()]);
        if (code !== 0) {
            throw UpdateError.of(
                "COMMAND_FAILED",
                `${action} failed with status ${code}: ${error.trim()}`,
            );
        }
    }
}
