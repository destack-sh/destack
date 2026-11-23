import type { CommandBody } from "../command/command.ts";
import type { CommandEnvironment } from "../command/context.ts";
import { CliError } from "../error/index.ts";

/** The capability git announces since 2.46, with which a helper marks a credential ephemeral. */
const AUTHTYPE_CAPABILITY = "capability[]=authtype";

/** The user name beside a token in Git's Basic authentication, as GitHub's tokens take. */
const TOKEN_USER = "x-access-token";

/** The prefix of the capabilities git lists in a request, each its own line. */
const CAPABILITY_PREFIX = "capability[]=";

/** The variables selecting a profile, its universe and its device, which git's helper runs with. */
const SELECTING_VARIABLES = [
    "DESTACK_PROFILE",
    "DESTACK_UNIVERSE_ORIGIN",
    "DESTACK_DIRECTORY",
    "DESTACK_DISTRIBUTION",
] as const;

/** Push and pull Destack repositories with git as the person signed in. */
export const gitCommand: CommandBody = {
    commands: {
        setup: {
            description: "Register destack as git's credential helper, as gh auth setup-git does",
            async run(_call, context) {
                // replace any other helper of the universe's origin, which serves every repository, with this program
                const { universe } = (await context.launcher()).session;
                const key = `credential.${new URL(universe).origin}.helper`;
                const helper = `!${credentialCommand(context.environment)} git credential`;
                await configure(["--replace-all", key, ""], context.environment);
                await configure(["--add", key, helper], context.environment);
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

                // answer with the person's account token, which the universe exchanges for the repository's
                const launcher = await context.launcher();
                const token = await launcher.session.credential.accountToken();
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

    /** Answer the request with a token as a password, which git sends with every request. */
    answer(token: string): string {
        // answer a user and the token as its password, as git sends Basic credentials with a large push's probe
        const credential = [`username=${TOKEN_USER}`, `password=${token}`];

        // ask a git with the capability to keep the token out of its other helpers
        return this.fields.has(AUTHTYPE_CAPABILITY)
            ? [AUTHTYPE_CAPABILITY, ...credential, "ephemeral=true"].join("\n")
            : credential.join("\n");
    }
}

/** Write the command running this program on its universe, as gh names its own path. */
function credentialCommand(environment: CommandEnvironment): string {
    // run the standalone program, or the runtime with its script from where its configuration lies
    const { process } = environment;
    const program = process.runtime.isStandalone
        ? [process.executable]
        : [process.executable, "--cwd", process.workingDirectory, process.script];

    // pass the variables selecting the universe and the device, as git runs helpers without them
    const selecting = SELECTING_VARIABLES.flatMap((name) => {
        const value = process.variables[name];

        return value === undefined ? [] : [`${name}=${quote(value)}`];
    });
    const prefix = selecting.length === 0 ? "" : `env ${selecting.join(" ")} `;

    return `${prefix}${program.map(quote).join(" ")}`;
}

/** Quote a word for the shell git runs a helper in. */
function quote(word: string): string {
    return `'${word.replaceAll("'", "'\\''")}'`;
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
