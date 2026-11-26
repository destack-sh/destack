import { expect, refusal, test } from "@destack/test";
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

test("ask a new person's region and name to sign them up by handle", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];
    const prompt = answering(asked, "eu", "Bea");

    // sign Bea up by her handle
    const signUp = await new SignInPrompt({ handle: "bea" }, prompt, environment()).signUp(regions);

    // take the time zone and locale this terminal runs in
    const { timeZone, locale } = environment();
    expect({ signUp, asked }).toEqual({
        signUp: { name: "Bea", handle: "bea", regionId: "eu", timeZone, locale },
        asked: ["Region: eu (Europe)", "Name"],
    });
});

test("refuse a sign-up's unknown region and empty name", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];

    // sign up on another region, then with an empty name
    const region = await refusal(
        new SignInPrompt({ handle: "bea", region: "mars" }, answering(asked), environment()).signUp(
            regions,
        ),
    );
    const name = await refusal(
        new SignInPrompt(
            { handle: "bea", region: "eu" },
            answering(asked, ""),
            environment(),
        ).signUp(regions),
    );
    expect({ region, name, asked }).toEqual({
        region: [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "region 'mars' is invalid: choose one of 'eu (Europe)'",
        ],
        name: [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "name is invalid: it has 0 characters, not 1 to 200",
        ],
        asked: ["Name"],
    });
});

test("sign in without a sign-up when no handle is given", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];

    // read the sign-up of a person giving only an email
    const signUp = await new SignInPrompt({ email: EMAIL }, answering(asked), environment()).signUp(
        regions,
    );
    expect({ signUp, asked }).toEqual({ signUp: undefined, asked: [] });
});

test("read the email by flag, else from the environment, else by asking", async () => {
    // answer each question in turn, recording it
    const asked: string[] = [];
    const prompt = (...answers: string[]) => answering(asked, ...answers);

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

/** Answer each question in turn with the next answer, recording it in asked. */
function answering(asked: string[], ...answers: string[]) {
    return {
        ask: async (question: string) => {
            asked.push(question);

            return answers.shift() ?? "";
        },
    };
}

/** List the universe's one region. */
async function regions() {
    return [{ code: "eu", name: "Europe" }];
}
