import { type Environment, FileSystemError } from "@destack/environment";
import { isAbsolute, join, resolve } from "node:path";
import { schema, Uuid } from "@destack/schema";
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
    /** The files and randomness the registration is kept with. */
    readonly environment: Environment<"files">;

    /** Select the user's installation directory, kept through an environment's files. */
    constructor(directory: string, environment: Environment<"files">) {
        this.directory = resolve(directory);
        this.environment = environment;
    }

    /** Read the registered application, absent before installation. */
    async read(): Promise<InstallationRecord | undefined> {
        // read the atomic registration document, absent before installation
        const { files } = this.environment;
        const path = join(this.directory, "installation.json");
        const text = await files.readTextFile(path).catch(FileSystemError.absent);

        return text === undefined ? undefined : this.validate(JSON.parse(text));
    }

    /** Record a verified application without changing its installer ownership. */
    async register(record: InstallationRecord): Promise<void> {
        // reject implicit conversion between independently managed installations
        this.validate(record);
        const current = await this.read();
        if (current !== undefined && current.method !== record.method) {
            throw UpdateError.of(
                "INSTALLATION_METHOD_MISMATCH",
                "uninstall the existing installation before changing its method",
            );
        }

        // publish the complete registration in one rename
        const { files, random } = this.environment;
        await files.mkdir(this.directory, { recursive: true, mode: 0o700 });
        const pending = join(this.directory, `installation.${Uuid.v4(random)}.json`);
        await files.writeTextFile(pending, JSON.stringify(record) + "\n", {
            createNew: true,
            mode: 0o600,
        });
        await files.rename(pending, join(this.directory, "installation.json"));
    }

    /** Parse a registration, rejecting unknown installers and relative application locations. */
    private validate(value: unknown): InstallationRecord {
        // validate persisted registration before using it for installation operations
        const record = InstallationRecord.safeParse(value);
        if (!record.success || !isAbsolute(record.data.application)) {
            throw UpdateError.of("RECORD_INVALID", "invalid application registration");
        }

        return record.data;
    }
}
