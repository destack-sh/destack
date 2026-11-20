import { DOMAINS } from "@destack/account/address";
import { Universe } from "@destack/update/release";
import { PackageMount, SpaceMount } from "@destack/object";
import { PackageId } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { MACHINE_PORT } from "@destack/machine/computer";
import type { Command } from "../command/command.ts";
import type { CommandEnvironment } from "../command/context.ts";
import { CliError } from "../error/index.ts";

/** The capability git announces to helpers answering an authentication scheme, since 2.46. */
const AUTHTYPE_CAPABILITY = "capability[]=authtype";

/** The prefix of the capabilities git lists in a request, each its own line. */
const CAPABILITY_PREFIX = "capability[]=";

/** The variable moving the local machine off its fixed port. */
const PORT_VARIABLE = "DESTACK_PORT";

/** Push and pull Destack repositories with git as the person signed in. */
export const gitCommand: Command = {
    description: "Push and pull Destack repositories with git as you",
    commands: {
        setup: {
            description: "Register destack as git's credential helper, as gh auth setup-git does",
            async run(_call, context) {
                // replace any other helper of each machine origin with this one, reading the path
                const helper = `!${Universe.current(context.environment).command} git credential`;
                for (const origin of GitCredentialRequest.origins(context.environment)) {
                    const key = `credential.${origin}`;
                    await configure(["--replace-all", `${key}.helper`, ""], context.environment);
                    await configure(["--add", `${key}.helper`, helper], context.environment);
                    await configure([`${key}.useHttpPath`, "true"], context.environment);
                }
                await context.terminal.print(
                    "git now pushes and pulls Destack repositories as you",
                );
            },
        },
        credential: {
            description: "Answer git's credential requests with your tokens",
            arguments: [{ name: "operation", usage: "get|store|erase" }],
            isHidden: true,
            async run({ arguments: { operation } }, context) {
                // read git's request, leaving storing and erasing to the tokens' own expiry
                const { stdin } = context.environment.process;
                const request = GitCredentialRequest.parse(
                    await new Response(stdin.readable).text(),
                );
                if (operation !== "get") {
                    return;
                }

                // issue the person's token for the package of the space the path names
                const { spaceId, packageId } = request.mount();
                const launcher = await context.launcher();
                const token = await launcher.session.credential.token(packageId, {
                    partition: spaceId,
                });
                await context.terminal.print(request.answer(token));
            },
        },
    },
};

/** A credential request git sends its helper: key=value lines, a line per capability. */
export class GitCredentialRequest {
    /** The request's fields by key, each capability as a key without a value. */
    readonly fields: ReadonlyMap<string, string>;

    /** Take a request's fields. */
    constructor(fields: ReadonlyMap<string, string>) {
        this.fields = fields;
    }

    /** Read a request as git writes it to a helper's standard input (gitcredentials(7)). */
    static parse(text: string): GitCredentialRequest {
        const fields = text
            .split("\n")
            .filter((line) => line !== "")
            .map((line): [string, string] => {
                const separator = line.indexOf("=");

                return line.startsWith(CAPABILITY_PREFIX)
                    ? [line, ""]
                    : [line.slice(0, separator), line.slice(separator + 1)];
            });

        return new GitCredentialRequest(new Map(fields));
    }

    /** List the origins Destack machines answer at: the cloud's, and this computer's machine. */
    static origins(environment: CommandEnvironment): readonly string[] {
        const port = environment.process.variables[PORT_VARIABLE] ?? String(MACHINE_PORT);

        return [`https://*.${DOMAINS.machine}`, `http://127.0.0.1:${port}`];
    }

    /** Read the space and the package the request's path names below its machine. */
    mount(): { readonly spaceId: Identifier<"space">; readonly packageId: PackageId } {
        // read the package below the space, as git sends the path without its leading slash
        const path = `/${this.fields.get("path") ?? ""}`;
        const space = SpaceMount.parse(path);
        const mounted = space === undefined ? undefined : PackageMount.parse(space.path);
        if (space === undefined || mounted === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `path '${path}' is invalid: it names no package of a space`,
            );
        }

        return {
            spaceId: schema.identifier("space").parse(space.spaceId),
            packageId: PackageId.parse(mounted.packageId),
        };
    }

    /** Answer the request with a Bearer token, refusing a git without the authtype capability. */
    answer(token: string): string {
        // require git's authtype capability, which carries a Bearer token
        if (!this.fields.has(AUTHTYPE_CAPABILITY)) {
            throw CliError.of(
                "GIT_VERSION_INVALID",
                "git version is invalid: Destack's credentials need git 2.46 or later",
            );
        }

        return [
            AUTHTYPE_CAPABILITY,
            "authtype=Bearer",
            `credential=${token}`,
            "ephemeral=true",
        ].join("\n");
    }
}

/** Set a key of git's global configuration, failing with git's message. */
async function configure(words: readonly string[], environment: CommandEnvironment): Promise<void> {
    const command = ["git", "config", "--global", ...words];
    const { code, stderr } = await environment.process.run(command);
    if (code !== 0) {
        throw CliError.of(
            "GIT_SETUP_BROKEN",
            `'${command.join(" ")}' exited with status ${code}: '${new TextDecoder().decode(stderr).trim()}'`,
        );
    }
}
