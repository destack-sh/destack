import { OWNER_EMAIL, SCENARIO_MILLISECONDS, TestComputer } from "@destack/machine/test";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Stage } from "@destack/update/release";
import { Link } from "../link/index.ts";
import { openShell } from "../test/index.ts";
import { Desktop } from "./desktop.ts";
import { DESKTOP_OBJECTS } from "../object/index.ts";

/** The fields of a command the desktop sends its shell that the test reads. */
const Sent = schema.looseObject({
    /** The command's action. */
    action: schema.string(),
    /** The kind of a window it opens. */
    kind: schema.string().exactOptional(),
    /** The title of a window it opens. */
    title: schema.string().exactOptional(),
    /** The address of a window it opens. */
    url: schema.string().exactOptional(),
    /** The entries of a menu it sets. */
    items: schema
        .array(schema.looseObject({ title: schema.string().exactOptional() }))
        .exactOptional(),
});

/** Report whether a command sets a menu offering an entry of a title. */
function isOffered(title: string): (command: unknown) => boolean {
    return (command) => {
        const { action, items = [] } = Sent.parse(command);

        return action === "menu.set" && items.some((item) => item.title === title);
    };
}

/** Report whether a command is of an action. */
function isAction(action: string): (command: unknown) => boolean {
    return (command) => Sent.parse(command).action === action;
}

test(
    "show a person another program signs in after the desktop started, once their sign-in enrolled the machine, and offer signing in once they sign out",
    async () => {
        // run the owner's home in the cloud, and their laptop with the desktop's objects through the relay
        const { home } = await TestComputer.home();
        const { world } = home;
        const relay = await TestComputer.relay(world);
        const laptop = await TestComputer.open({
            world,
            relay: relay.url,
            localObjects: DESKTOP_OBJECTS,
        });
        const native = laptop.device;

        // start the desktop, which asks for a browser sign-in while nobody is signed in
        const { shell, sent, next, emit, close } = openShell();
        const environment = { ...laptop.environment, process: testHost().process };
        const desktop = await Desktop.open(native, shell, Link.of(new Stage("local")), environment);
        const running = desktop.run(undefined);
        await world.simulation.wait(next(isAction("browser.open")));

        // sign in by mail and enroll the laptop as the CLI does, and see the desktop show them and their start page
        const shown = next(isOffered("Sign Out"));
        const opened = next(isAction("window.open"));
        const signedIn = await laptop.signIn(OWNER_EMAIL);
        await laptop.enroll(signedIn, world.owner.account);
        await world.simulation.wait(Promise.all([shown, opened]));

        // press the palette's shortcut, and see the palette float over the start page
        const floated = next(isAction("window.open"));
        emit({ action: "shortcut.pressed", id: "palette" });
        const { kind, title, url = "" } = Sent.parse(await world.simulation.wait(floated));
        const { origin, pathname } = new URL(url);

        // sign them out as the CLI does, and see the desktop offer signing in
        const offered = next(isOffered("Sign In…"));
        await native.signOut((await native.session(signedIn.id)).login);
        await world.simulation.wait(offered);

        // stop the desktop
        close();
        await world.simulation.wait(running);
        await desktop[Symbol.asyncDispose]();
        const actions = sent.map((command) => Sent.parse(command).action);
        const tray = sent.filter(isAction("tray.set"));

        // expect the palette over the start space, the tray serving over the open tunnel, and every command in order
        expect({
            palette: [kind, title, `${origin}${pathname}`],
            tray,
            actions,
            last: sent.at(-1),
        }).toEqual({
            palette: [
                "palette",
                "Commands",
                "https://start--home--account-39751c3a0e7f.destack.space/command",
            ],
            tray: [
                { action: "tray.set", tooltip: "Destack: ⁨Serving 0 spaces⁩" },
                { action: "tray.set", tooltip: "Destack: ⁨Serving 0 spaces⁩" },
                { action: "tray.set", tooltip: "Destack" },
            ],
            actions: [
                "menu.title",
                "autostart.set",
                "menu.set",
                "menu.set",
                "browser.open",
                "menu.set",
                "menu.set",
                "menu.set",
                "menu.set",
                "menu.set",
                "tray.set",
                "shortcut.register",
                "badge.set",
                "window.open",
                "window.open",
                "tray.set",
                "window.close",
                "window.close",
                "badge.set",
                "menu.set",
                "tray.set",
                "shortcut.unregister",
                "menu.set",
                "menu.set",
                "menu.set",
            ],
            last: {
                action: "menu.set",
                menu: "tray",
                items: [
                    {
                        kind: "toggle",
                        id: "autostart.toggle",
                        title: "Open at Login",
                        isChecked: true,
                    },
                    { kind: "separator" },
                    { kind: "action", id: "account.signIn", title: "Sign In…" },
                ],
            },
        });
    },
    SCENARIO_MILLISECONDS,
);
