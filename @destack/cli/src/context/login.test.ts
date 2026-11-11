import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";
import { SignInPrompt } from "./login.ts";
import { testHost } from "@destack/test/host";

/** The email a person signs in with. */
const EMAIL = "ada@example.test";

/** A terminal's environment with some environment variables, in UTC and US English. */
function environment(variables: Readonly<Record<string, string>> = {}) {
    return {
        process: TerminalFixture.process(variables, testHost().scheduler),
        timeZone: "UTC",
        locale: "en-US",
    };
}

test("read a new person's sign-up from their handle, asking the residency and name, refusing an empty name, and nothing without a handle", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];
    const prompt = (...answers: string[]) => ({
        ask: async (question: string) => {
            asked.push(question);

            return answers.shift() ?? "";
        },
    });

    // sign in without a handle, sign up asking the residency and name, and refuse another residency
    const signIn = await new SignInPrompt({ email: EMAIL }, prompt(), environment()).signUp(
        residencies,
    );
    const signUp = await new SignInPrompt(
        { handle: "bea" },
        prompt("eu", "Bea"),
        environment(),
    ).signUp(residencies);
    const refused = await new SignInPrompt(
        { handle: "bea", residency: "mars" },
        prompt(),
        environment(),
    )
        .signUp(residencies)
        .catch((error: unknown) => (error instanceof Error ? error.message : error));
    const unnamed = await new SignInPrompt(
        { handle: "bea", residency: "eu" },
        prompt(""),
        environment(),
    )
        .signUp(residencies)
        .catch((error: unknown) => (error instanceof Error ? error.message : error));

    // take the time zone and locale this terminal runs in
    const { timeZone, locale } = environment();
    expect({ signIn, signUp, refused, unnamed, asked }).toEqual({
        signIn: undefined,
        signUp: { name: "Bea", handle: "bea", residencyId: "eu", timeZone, locale },
        refused: "residency 'mars' is invalid: choose one of 'eu (Europe)'",
        unnamed: "name is invalid: it has 0 characters, not 1 to 200",
        asked: ["Residency: eu (Europe)", "Name", "Name"],
    });
});

test("read the email by flag, else from the environment, else by asking", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];
    const prompt = (...answers: string[]) => ({
        ask: async (question: string) => {
            asked.push(question);

            return answers.shift() ?? "";
        },
    });

    // read the email by flag, then by asking, then from the environment
    const flagged = await new SignInPrompt({ email: EMAIL }, prompt(), environment()).email();
    const typedIn = await new SignInPrompt({}, prompt("bea@example.test"), environment()).email();
    const scripted = await new SignInPrompt(
        {},
        prompt(),
        environment({ DESTACK_EMAIL: "grace@example.test" }),
    ).email();
    expect({ flagged, typedIn, scripted, asked }).toEqual({
        flagged: EMAIL,
        typedIn: "bea@example.test",
        scripted: "grace@example.test",
        asked: ["Email"],
    });
});

/** List the universe's one residency. */
async function residencies() {
    return [{ code: "eu", name: "Europe" }];
}
