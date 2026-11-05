import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { expect, test } from "@destack/test";
import { Desktop } from "./desktop.ts";

/** The person signed in. */
const LOGIN = { name: "Florian" };

test("offer signing out or switching while signed in, signing in once no sign-in runs, each running its verb", async () => {
    // list the entries of each status, recording the verb each runs
    const ran: string[] = [];
    const desktop = {
        signIn: async () => void ran.push("signIn"),
        signOut: async () => void ran.push("signOut"),
        switch: async () => void ran.push("switch"),
        localization: Localization.of(SOURCE_LOCALE, []),
    };
    const sections = [
        Desktop.accountEntries({ status: "signedIn", login: LOGIN }, desktop),
        Desktop.accountEntries({ status: "signingIn" }, desktop),
        Desktop.accountEntries({ status: "signedOut" }, desktop),
    ];
    for (const entry of sections.flat()) {
        if (entry.kind === "action") {
            await entry.run();
        }
    }

    // expect the person's name and the actions, and each action running its verb
    expect({
        sections: sections.map((entries) => entries.map((entry) => [entry.kind, entry.title])),
        ran,
    }).toEqual({
        sections: [
            [
                ["label", "Florian"],
                ["action", "Switch Account…"],
                ["action", "Sign Out"],
            ],
            [["label", "Signing In…"]],
            [["action", "Sign In…"]],
        ],
        ran: ["switch", "signOut", "signIn"],
    });
});
