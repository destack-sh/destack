import { mkdir, rename, writeFile } from "node:fs/promises";
import { readOptional } from "@destack/fs";
import { isAbsolute, join, resolve } from "node:path";
import { schema } from "@destack/schema";
import { UpdateError } from "../error/index.ts";

/** The installer responsible for application files and their removal. */
export const InstallationMethod = schema.enum(["archive", "application", "nsis"]);

/** The installer responsible for application files and their removal. */
export type InstallationMethod = schema.Infer<typeof InstallationMethod>;

/** The registered application and its installation method. */
export const InstallationRecord = schema.object({
    /** Installer responsible for this application. */
    method: InstallationMethod,
    /** Absolute path to the installed application bundle or directory. */
    application: schema.string(),
});

/** The registered application and its installation method. */
export type InstallationRecord = schema.Infer<typeof InstallationRecord>;

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
        const source = await readOptional(join(this.directory, "installation.json"));
        if (source === undefined) {
            return undefined;
        }

        return this.validate(JSON.parse(source));
    }

    /** Record a verified application without changing its installer ownership. */
    async register(record: InstallationRecord): Promise<void> {
        // reject implicit conversion between independently managed installations
        this.validate(record);
        const current = await this.read();
        if (current !== undefined && current.method !== record.method) {
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

    /** Parse a registration, rejecting unknown installers and relative application locations. */
    private validate(value: unknown): InstallationRecord {
        // validate persisted registration before using it for installation operations
        const record = InstallationRecord.safeParse(value);
        if (!record.success || !isAbsolute(record.data.application)) {
            throw new UpdateError("INSTALL", "invalid application registration");
        }

        return record.data;
    }
}
