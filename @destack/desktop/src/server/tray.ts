import { argument, type Localization, plural, t, verbatim } from "@destack/locale";
import type { SpaceAddress } from "@destack/account/address";
import { machine, placement, type Space, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import type { COMPUTER_TUNNELS, computer, Launcher } from "@destack/machine/device";
import { type ObjectClient, RelationalQuery } from "@destack/object/client";
import { type Identifier, present } from "@destack/schema";
import type { Shell } from "../shell/index.ts";
import { Menu, type MenuEntry } from "./menu.ts";

/** The status line of a computer serving spaces. */
const SERVING = t`${plural(argument("count"), { one: "Serving # space", other: "Serving # spaces" })}`;

/** The status line of a computer moving spaces to or from it. */
const SYNCING = t`${plural(argument("count"), { one: "Syncing # space", other: "Syncing # spaces" })}`;

/** The tray's tooltip: the brand and a status line. */
const TOOLTIP = t`Destack: ${argument("line")}`;

/** How this computer serves its spaces, as its tray shows it. */
export type TrayStatus = "serving" | "syncing" | "paused" | "offline" | "error";

/** Whether a computer's relay tunnel runs, as its record states. */
type ComputerTunnel = (typeof COMPUTER_TUNNELS)[number];

/** A machine as the tray reads it: whether it stands and serves. */
type TrayMachine = Pick<Select<typeof machine.table>, "status" | "revokedAt">;

/** A placement as the tray reads it: its space's name and title, and the machine it moves to. */
type TrayPlacement = Pick<Select<typeof placement.table>, "spaceId" | "targetId"> & {
    readonly space: Pick<Space, "name" | "title"> | null;
};

/** What the tray reads of this computer: its machine, its tunnel and its placements. */
export interface TrayRead {
    /** This computer's machine, absent once it left its account. */
    readonly machine: TrayMachine | undefined;
    /** Whether the machine's relay tunnel runs, as this computer's record states. */
    readonly tunnel: ComputerTunnel;
    /** The placements on the machine and moving to it, each with its space. */
    readonly placements: readonly TrayPlacement[];
}

/** What the tray shows of this computer: its status, the text on hover and the items. */
export interface TrayView {
    /** The status. */
    readonly status: TrayStatus;
    /** The text the tray icon shows on hover. */
    readonly tooltip: string;
    /** The status line and the spaces. */
    readonly entries: readonly MenuEntry[];
}

/** Show how this computer serves its spaces in the tray. */
export class Tray {
    /** This computer's machine and the account's spaces, replicated as the person. */
    readonly #account: ObjectClient<{
        readonly machine: typeof machine;
        readonly placement: typeof placement;
        readonly space: typeof space;
    }>;
    /** The launcher of the person signed in, finding the spaces to open. */
    readonly #launcher: Pick<Launcher, "address">;
    /** This computer's machine. */
    readonly #machineId: Identifier<"machine">;
    /** This computer's record, stating its tunnel. */
    readonly #computers: ObjectClient<{ readonly computer: typeof computer }>;
    /** The menus showing the items. */
    readonly #menu: Menu;
    /** The native shell showing the status on the tray icon. */
    readonly #shell: Shell;
    /** The language the tray shows in. */
    readonly #localization: Localization;
    /** Float the command palette over a space. */
    readonly #open: (space: SpaceAddress) => Promise<void>;

    /** Show this computer's machine from the account's objects in the tray. */
    constructor(options: {
        readonly account: ObjectClient<{
            readonly machine: typeof machine;
            readonly placement: typeof placement;
            readonly space: typeof space;
        }>;
        readonly computers: ObjectClient<{ readonly computer: typeof computer }>;
        readonly launcher: Pick<Launcher, "address">;
        readonly machineId: Identifier<"machine">;
        readonly menu: Menu;
        readonly shell: Shell;
        readonly localization: Localization;
        readonly open: (space: SpaceAddress) => Promise<void>;
    }) {
        // set the clients, the launcher, the machine, the menus, the shell, the language and the palette
        this.#account = options.account;
        this.#computers = options.computers;
        this.#launcher = options.launcher;
        this.#machineId = options.machineId;
        this.#menu = options.menu;
        this.#shell = options.shell;
        this.#localization = options.localization;
        this.#open = options.open;
    }

    /** Read the status of this computer's machine, its tunnel and the placements on it. */
    static status(read: TrayRead): TrayStatus {
        // fail without a standing machine
        const { machine: row, placements } = read;
        if (row === undefined || row.revokedAt !== null) {
            return "error";
        }
        // pause while the machine drains or is disabled
        else if (row.status !== "enabled") {
            return "paused";
        }
        // go offline while peers cannot reach the machine through its relay
        else if (read.tunnel !== "open") {
            return "offline";
        }
        // sync while a space moves to or from the machine
        else if (placements.some((placed) => placed.targetId !== null)) {
            return "syncing";
        }
        // serve otherwise
        else {
            return "serving";
        }
    }

    /** Plan what the tray shows of this computer's machine, its placements and their spaces. */
    static view(
        read: TrayRead,
        open: (spaceId: Identifier<"space">) => Promise<void>,
        localization: Localization,
    ): TrayView {
        // describe the status in the host's language
        const status = Tray.status(read);
        const count = { count: read.placements.length };
        const line = {
            serving: localization.render(SERVING, count),
            syncing: localization.render(SYNCING, count),
            paused: localization.render(t`Paused`),
            offline: localization.render(t`Offline`),
            error: localization.render(t`This computer left its account`),
        }[status];

        // list the spaces by title, marking the ones moving
        const served = read.placements
            .map((placed) => {
                const { title, name } = present(placed.space, `space ${placed.spaceId}`);

                return { placed, title: title ?? name };
            })
            .toSorted((left, right) => left.title.localeCompare(right.title))
            .map(({ placed, title }): MenuEntry => ({
                kind: "action",
                id: placed.spaceId,
                title:
                    placed.targetId === null
                        ? title
                        : localization.render(t`${argument("title")} (Syncing)`, { title }),
                run: () => open(placed.spaceId),
            }));
        const entries: MenuEntry[] = [{ kind: "label", title: line }, ...served];

        return { status, tooltip: localization.render(TOOLTIP, { line }), entries };
    }

    /** Show the tray as the account's rows change, until aborted or a client fails. */
    async follow(signal: AbortSignal): Promise<void> {
        // watch the machine, its record, and the placements on it and moving to it with their spaces
        const machineId = this.#machineId;
        const account = this.#account;
        const computers = this.#computers;
        const followed = RelationalQuery.latest(
            {
                machine: account.query.machine.findFirst({ where: { id: machineId } }),
                record: computers.query.computer.findFirst({ where: { machineId } }),
                placements: account.query.placement.findMany({
                    where: { OR: [{ machineId }, { targetId: machineId }] },
                    with: { space: true },
                }),
            },
            signal,
        );

        // show each change
        for await (const { machine: row, record, placements } of followed) {
            const { tunnel } = present(record, `the record of machine ${machineId}`);
            await this.show({ machine: row, tunnel, placements });
        }
    }

    /** Show the machine's status and its spaces in the tray. */
    async show(read: TrayRead): Promise<void> {
        // send the section and the tooltip
        const open = async (spaceId: Identifier<"space">) =>
            this.#open(await this.#launcher.address(spaceId));
        const view = Tray.view(read, open, this.#localization);
        await this.#menu.set("serving", view.entries);
        await this.#shell.send({ action: "tray.set", tooltip: view.tooltip });
    }

    /** Show that the account is out of reach, in the status line and on hover. */
    async failed(): Promise<void> {
        // send the line in the section and the tooltip
        const line = this.#localization.render(t`Cannot reach your account`);
        await this.#menu.set("serving", [{ kind: "label", title: line }]);
        await this.#shell.send({
            action: "tray.set",
            tooltip: this.#localization.render(TOOLTIP, { line }),
        });
    }

    /** Remove the serving section, as nobody is signed in to show it for. */
    async clear(): Promise<void> {
        // empty the section and the tooltip
        await this.#menu.set("serving", []);
        await this.#shell.send({ action: "tray.set", tooltip: verbatim`Destack` });
    }
}
