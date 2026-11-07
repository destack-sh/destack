import { currentLogin, currentSpace } from "@destack/account/native";
import { Account } from "@destack/account/object";
import { OWNER_EMAIL } from "@destack/machine/test";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** The email of a second person signed in on the computer. */
const OTHER_EMAIL = "bea@example.test";

/** A printed login's name, status and space. */
const Person = schema.looseObject({
    name: schema.string(),
    status: schema.string(),
    space: schema.string().nullable(),
});

/** A printed list of spaces, by name. */
const Listed = schema.array(schema.looseObject({ name: schema.string() }));

/** Type a mailed code as it was mailed. */
function typed(code: string): string {
    return code;
}

test("sign a person in with a mailed code, and switch to their home when no space is set", async () => {
    // sign the owner in again, then once more after they switched to a space
    const terminal = await TerminalFixture.open();
    const { native } = terminal;
    const first = await terminal.signIn(["login", "--email", OWNER_EMAIL], OWNER_EMAIL, [], typed);
    await native.set(currentSpace, terminal.computer.world.owner.home);
    const second = await terminal.signIn(
        ["login", "--email", OWNER_EMAIL, "--json"],
        OWNER_EMAIL,
        [],
        typed,
    );
    const stored = await native.logins.query.login.findMany();

    // switch to the home once, keeping one login for the person
    expect({ first, second, stored: stored.map((row) => [row.id, row.subjectId]) }).toEqual({
        first: {
            code: 0,
            output: `Signed in as ${terminal.login.name}.\nSwitched to ${terminal.address("home")}.\n`,
            error: `Sent a code to ${OWNER_EMAIL}.\n`,
        },
        second: {
            code: 0,
            output: `${JSON.stringify({ login: stored[0], space: null })}\n`,
            error: `Sent a code to ${OWNER_EMAIL}.\n`,
        },
        stored: [[terminal.login.id, terminal.login.subjectId]],
    });
});

test("act as a person signing in beside another, in their home instead of the other's space", async () => {
    // leave a second person signed in, acting in their home
    const terminal = await TerminalFixture.open();
    const { native } = terminal;
    const other = await terminal.computer.signIn(OTHER_EMAIL);
    await native.set(currentLogin, other.id);
    await native.set(currentSpace, Account.home(Account.personal(other.subjectId)));

    // sign the owner in, and act as them in their home
    const signedIn = await terminal.signIn(
        ["login", "--email", OWNER_EMAIL],
        OWNER_EMAIL,
        [],
        (code) => code,
    );
    expect({
        output: signedIn.output,
        login: await native.setting(currentLogin),
        space: await native.setting(currentSpace),
    }).toEqual({
        output: `Signed in as ${terminal.login.name}.\nSwitched to ${terminal.address("home")}.\n`,
        login: terminal.login.id,
        space: terminal.computer.world.owner.home,
    });
});

test("refuse a sign-in the universe refused, with its reason", async () => {
    // sign in typing a code nobody mailed
    const terminal = await TerminalFixture.open();
    const refused = await terminal.signIn(
        ["login", "--email", OWNER_EMAIL],
        OWNER_EMAIL,
        [],
        () => "000000",
    );
    expect(refused).toEqual({
        code: 1,
        output: "",
        error: [
            `Sent a code to ${OWNER_EMAIL}.`,
            "destack: the token endpoint refused the request: invalid_grant: the code is wrong or expired",
            "",
        ].join("\n"),
    });
});

test("sign a person out of this computer", async () => {
    // sign out, reporting the login signed out
    const terminal = await TerminalFixture.open();
    expect([
        await terminal.printed(["logout", "--json"]),
        await terminal.native.logins.query.login.findMany(),
    ]).toEqual([
        { code: 0, output: `${JSON.stringify({ id: terminal.login.id })}\n`, error: "" },
        [],
    ]);
});

test("sign a new person up with a handle and a residency, then show who they are and their spaces", async () => {
    // sign Ada up with a mailed code, then show the login and list the spaces
    const terminal = await TerminalFixture.open();
    const email = "ada@example.test";
    const words = [
        "login",
        "--email",
        email,
        "--handle",
        "ada",
        "--residency",
        "eu",
        "--name",
        "Ada",
    ];
    const signedUp = await terminal.signIn(words, email, [], typed);
    const whoami = await terminal.printed(["whoami", "--json"]);
    const spaces = await terminal.printed(["spaces", "--json"]);
    const person = Person.parse(JSON.parse(whoami.output));
    const listed = Listed.parse(JSON.parse(spaces.output));

    // sign Ada up into a new home, signed in there, which is the one space
    expect({
        signedUp,
        whoami: [whoami.code, whoami.error, person.name, person.status, person.space],
        spaces: [spaces.code, spaces.error, listed.map((space) => space.name)],
    }).toEqual({
        signedUp: {
            code: 0,
            output: "Signed in as Ada.\nSwitched to home--ada.\n",
            error: `Sent a code to ${email}.\n`,
        },
        whoami: [0, "", "Ada", "authenticated", "home--ada"],
        spaces: [0, "", ["home"]],
    });
});
