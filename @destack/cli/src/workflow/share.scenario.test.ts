import { currentLogin, currentSpace } from "@destack/account/native";
import { Account } from "@destack/account/object";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { TerminalFixture } from "../test/terminal.ts";

/** The email of the person the owner shares a space with. */
const OTHER_EMAIL = "other@example.test";

/** A printed invitation's identifier. */
const Invited = schema.looseObject({ id: schema.string() });

test("share a space with a person by email, who accepts it and then switches to it", async () => {
    // open the owner's home with a desk beside it, and sign a second person in on the computer
    const terminal = await TerminalFixture.open({ NO_COLOR: "1" });
    const { native } = terminal;
    await terminal.space("desk");
    const other = await terminal.computer.signIn(OTHER_EMAIL);

    // invite the person to the desk as an editor, acting as the owner
    await native.set(currentLogin, terminal.login.id);
    const words = ["--space", "desk", "share", OTHER_EMAIL, "--role", "editor", "--json"];
    const shared = await terminal.printed(words);
    expect([shared.code, shared.error]).toEqual([0, ""]);
    const invitation = Invited.parse(JSON.parse(shared.output));

    // act as the person in their home: list their spaces and invitations, accept, and list again
    await native.set(currentLogin, other.id);
    await native.set(currentSpace, Account.home(Account.personal(other.subjectId)));
    const before = await terminal.printed(["switch", "desk"]);
    const pending = await terminal.printed(["accept", "--json"]);
    const accepted = await terminal.printed(["accept", invitation.id]);
    const none = await terminal.printed(["accept"]);
    expect({
        shared: [shared.code, shared.error],
        pending: schema
            .array(Invited)
            .parse(JSON.parse(pending.output))
            .map((each) => each.id),
        accepted: [accepted.code, accepted.error],
        none: none.output,
        switched: [before.code, (await terminal.printed(["switch", "desk"])).output],
    }).toEqual({
        shared: [0, ""],
        pending: [invitation.id],
        accepted: [0, ""],
        none: "No invitations.\n",
        switched: [1, `Switched to ${terminal.address("desk")}.\n`],
    });
});
