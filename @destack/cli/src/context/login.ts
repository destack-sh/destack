import { currentLogin, currentSpace, type Login } from "@destack/account/native";
import { Account, DisplayName, NAME_LENGTH, type Residency } from "@destack/account/object";
import type { SignUp } from "@destack/account/better-auth";
import type { schema } from "@destack/schema";

import type { SpaceAddress } from "@destack/account/address";
import { type Device, type SignInMethod } from "@destack/machine/device";
import { type Verb, OptionValue, type Options } from "../command/command.ts";
import type { Environment } from "@destack/environment";
import { Prompt } from "../output/index.ts";
import type { Context } from "./context.ts";
import { CliError } from "../error/index.ts";

/** The environment variables a script answers sign-in with, by option. */
const ENVIRONMENT = {
    email: "DESTACK_EMAIL",
    handle: "DESTACK_HANDLE",
    residency: "DESTACK_RESIDENCY",
    name: "DESTACK_NAME",
} as const;

/** The environment values a sign-in's answers default to: its variables, time zone and locale. */
export type SignInPromptEnvironment = Pick<
    Environment<"process">,
    "process" | "timeZone" | "locale"
>;

/** The questions a sign-in asks: the email, the mailed code and a new person's sign-up. */
export class SignInPrompt {
    /** The command line's options. */
    readonly #options: Options;
    /** The lines a person types or a script pipes. */
    readonly #prompt: Pick<Prompt, "ask">;
    /** The environment whose variables, time zone and locale the answers default to. */
    readonly #environment: SignInPromptEnvironment;

    /** Answer from a command line's options, else ask through a prompt, in an environment. */
    constructor(
        options: Options,
        prompt: Pick<Prompt, "ask">,
        environment: SignInPromptEnvironment,
    ) {
        this.#options = options;
        this.#prompt = prompt;
        this.#environment = environment;
    }

    /** Report whether the person signs in with a mailed code: an email given. */
    isEmail(): boolean {
        return this.#given("email") !== undefined;
    }

    /** Read the email address. */
    async email(): Promise<string> {
        return this.#given("email") ?? (await this.#prompt.ask("Email"));
    }

    /** Read a new person's sign-up from their handle, nothing for a person signing in. */
    async signUp(
        residencies: () => Promise<readonly Pick<Residency, "code" | "name">[]>,
    ): Promise<schema.Input<typeof SignUp> | undefined> {
        // sign a person in without a handle
        const handle = this.#given("handle");
        if (handle === undefined) {
            return undefined;
        }

        // ask for the residency among the universe's, refusing another
        const offered = await residencies();
        const listed = offered
            .map((residency) => `${residency.code} (${residency.name})`)
            .join(", ");
        const residencyId =
            this.#given("residency") ?? (await this.#prompt.ask(`Residency: ${listed}`));
        if (!offered.some((residency) => residency.code === residencyId)) {
            throw CliError.of(
                "USAGE_INVALID",
                `unknown residency ${residencyId}: choose one of ${listed}`,
            );
        }

        // ask for the name, refusing one no account takes
        const name = this.#given("name") ?? (await this.#prompt.ask("Name"));
        if (!DisplayName.safeParse(name).success) {
            throw CliError.of(
                "USAGE_INVALID",
                `a name has 1 to ${NAME_LENGTH} characters, not ${name.length}`,
            );
        }

        // take the time zone and locale this terminal runs in
        const { timeZone, locale } = this.#environment;

        return { name, handle, residencyId, timeZone, locale };
    }

    /** Read the mailed code. */
    code(): Promise<string> {
        return this.#prompt.ask("Code");
    }

    /** Read an option's value, else its environment variable, absent for neither. */
    #given(option: keyof typeof ENVIRONMENT): string | undefined {
        const variable = this.#environment.process.variables[ENVIRONMENT[option]];

        return OptionValue.string(this.#options[option], option) ?? variable;
    }
}

/** Sign a person in with a device code, through the browser, or with a mailed code. */
export function loginCommand(context: Context): Verb {
    return {
        description:
            "Sign in or up with a device code, enrolling this machine; --browser signs in through the browser, --email with a mailed code, which --handle signs a new person up with",
        json: true,
        options: {
            browser: { type: "boolean", description: "Sign in through this computer's browser" },
            email: {
                type: "string",
                description: "Sign in with a code mailed to this address",
                argument: "email",
            },
            handle: {
                type: "string",
                description: "The handle a new person signs up with, such as florian",
                argument: "handle",
            },
            residency: {
                type: "string",
                description: "The jurisdiction a new person's account stores data in, such as eu",
                argument: "code",
            },
            name: {
                type: "string",
                description: "The name a new person signs up with",
                argument: "name",
            },
        },
        async run({ options, json }) {
            // read who the terminal acts as, then sign in and enroll this machine
            const client = await context.native();
            const acting = await client.login();
            const login = await signPersonIn(client, options, context);

            // act as the person signed in, in their home unless they acted in a space before
            await client.set(currentLogin, login.id);
            const switched = await switchHome(client, login, acting);
            const lines = [
                `Signed in as ${login.name}.`,
                ...(switched === undefined ? [] : [`Switched to ${switched}.`]),
            ];
            await context.terminal.print(
                json ? JSON.stringify({ login, space: switched ?? null }) : lines.join("\n"),
            );
        },
    };
}

/** Sign the person out of this machine. */
export function logoutCommand(context: Context): Verb {
    return {
        description: "Sign out of this machine",
        json: true,
        async run({ json }) {
            // revoke the session at its issuer and remove the login
            const login = await context.signedIn();
            await (await context.native()).signOut(login);
            await context.terminal.print(
                json ? JSON.stringify({ id: login.row.id }) : `Signed out ${login.row.name}.`,
            );
        },
    };
}

/** Show the person signed in on this machine and the space the terminal acts in. */
export function whoamiCommand(context: Context): Verb {
    return {
        description: "Show the person signed in on this machine and the space the terminal acts in",
        json: true,
        async run({ json }) {
            // read the person and the address of the space
            const launcher = await context.launcher();
            const { row } = await context.signedIn();
            const acting = await context.space();
            const space = acting === undefined ? null : await launcher.address(acting);
            await context.terminal.print(
                json ? JSON.stringify({ ...row, space }) : render(row, space),
            );
        },
    };
}

/** Sign a person in through the machine: in the browser, with a mailed code, or a device code. */
async function signPersonIn(
    client: Device,
    options: Options,
    context: Context,
): Promise<Login["row"]> {
    // read how the person signs in
    const { environment, terminal } = context;
    const prompt = new SignInPrompt(options, context.terminal.prompt, environment);
    const method = await signInMethod(client, options, prompt);

    // show each step the machine writes until it signs the person in
    return client.signIn(method, async (state, answer) => {
        // open the page in the browser
        if (state.kind === "open") {
            await terminal.printDiagnostic(`Opening ${state.url}`);
            await environment.opener.openUrl(state.url);
        }
        // show the code to enter at the page
        else if (state.kind === "enter") {
            await terminal.printDiagnostic(`First copy your one-time code: ${state.userCode}`);
            await terminal.printDiagnostic(`Then open ${state.url} to sign in.`);
        }
        // ask for the mailed code
        else if (state.kind === "waiting") {
            await terminal.printDiagnostic(`Sent a code to ${state.email}.`);
            await answer(await prompt.code());
        }
    });
}

/** Read how a person signs in: the browser, a mailed code with any sign-up, or a device code. */
async function signInMethod(
    client: Device,
    options: Options,
    prompt: SignInPrompt,
): Promise<SignInMethod> {
    // sign in through the browser
    if (options["browser"] === true) {
        return { kind: "browser" };
    }
    // sign in with a mailed code, or sign a new person up with their handle
    else if (prompt.isEmail()) {
        const email = await prompt.email();
        const signUp = await prompt.signUp(() => client.residencies());

        return signUp === undefined ? { kind: "email", email } : { kind: "signUp", email, signUp };
    }
    // sign in or up with a device code the person enters in any browser
    else {
        return { kind: "code" };
    }
}

/** Switch the terminal to a person's home when it acts in no space, returning the home. */
async function switchHome(
    client: Device,
    login: Login["row"],
    acting: Login["row"] | undefined,
): Promise<SpaceAddress | undefined> {
    // retain a space the same person switched to before
    const isSamePerson = acting?.subjectId === login.subjectId;
    if (isSamePerson && (await client.setting(currentSpace)) !== null) {
        return undefined;
    }

    // switch to the home of the person's account
    const launcher = await client.launcher(login.id);
    const home = Account.home(await launcher.account());
    await client.set(currentSpace, home);

    return launcher.address(home);
}

/** Render a login with its issuer and state, and the space the terminal acts in. */
function render(login: Login["row"], acted: string | null): string {
    return [
        `Name     ${login.name}`,
        `User     ${login.subjectId}`,
        `Issuer   ${login.issuer}`,
        `Session  ${login.status}`,
        `Space    ${acted ?? "none: destack switch <space>--<handle>"}`,
    ].join("\n");
}
