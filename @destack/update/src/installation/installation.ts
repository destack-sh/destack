import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { isAbsolute, join, resolve } from "node:path";
import { UpdateError } from "../error/index.ts";

/** The installer responsible for application files and their removal. */
export type InstallationMethod = "archive" | "application" | "nsis";

/** The registered application and its installation method. */
export interface InstallationRecord {
    /** Installer responsible for this application. */
    method: InstallationMethod;
    /** Absolute path to the installed application bundle or directory. */
    application: string;
}

/** Persistent application registration kept separately from versioned executable files. */
export class Installation {
    /** User directory containing registration and distribution state. */
    readonly directory: string;

    /** Select the user's installation directory. */
    constructor(directory: string) {
        this.directory = resolve(directory);
    }

    /** Read the registered application, absent before installation. */
    async read(): Promise<InstallationRecord | undefined> {
        // read the atomic registration document
        let source: string;
        try {
            source = await readFile(join(this.directory, "installation.json"), "utf8");
        } catch (error) {
            if ((error as NodeJS.ErrnoException).code === "ENOENT") {
                return undefined;
            }
            throw error;
        }
        const record = JSON.parse(source) as InstallationRecord;
        this.validate(record);

        return record;
    }

    /** Record a verified application without changing its installer ownership. */
    async register(record: InstallationRecord): Promise<void> {
        // reject implicit conversion between independently managed installations
        this.validate(record);
        const current = await this.read();
        if (current && current.method !== record.method) {
            throw new UpdateError(
                "INSTALL",
                "uninstall the existing installation before changing its method",
            );
        }

        // publish the complete registration in one rename
        await mkdir(this.directory, { recursive: true, mode: 0o700 });
        const pending = join(this.directory, `installation.${crypto.randomUUID()}.json`);
        await writeFile(pending, JSON.stringify(record) + "\n", { flag: "wx", mode: 0o600 });
        await rename(pending, join(this.directory, "installation.json"));
    }

    /** Reject unknown installers and relative application locations. */
    private validate(record: InstallationRecord): void {
        // validate persisted registration before using it for installation operations
        if (
            !record ||
            !["archive", "application", "nsis"].includes(record.method) ||
            typeof record.application !== "string" ||
            !isAbsolute(record.application)
        ) {
            throw new UpdateError("INSTALL", "invalid application registration");
        }
    }
}
