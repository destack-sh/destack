import { WindowBounds, type WindowRow } from "../object/index.ts";
import type { InterruptionLevel } from "@destack/notification/client";
import { schema } from "@destack/schema";

/** What a native window shows: a view or an object's opening, or the floating command palette. */
export type WindowKind = WindowRow["kind"];

/** A native menu the host fills: the tray's, or the application menu's first submenu. */
export type MenuName = "tray" | "application";

/** An action offered beside a banner, as the declaring notification lists it. */
export interface BannerAction {
    /** The action's declared name, which an act reports back. */
    readonly name: string;
    /** The button's label. */
    readonly title: string;
    /** Whether the action destroys or declines. */
    readonly isDestructive: boolean;
    /** The text field the action asks for. */
    readonly text?: { readonly placeholder: string; readonly button: string };
}

/** An item the host places in a native menu: a label, an action, a toggle or a separator. */
export type MenuItem =
    | {
          /** A line the person reads but cannot choose. */
          readonly kind: "label";
          /** The item's identity. */
          readonly id: string;
          /** The text. */
          readonly title: string;
      }
    | {
          /** An item the person chooses to run an action. */
          readonly kind: "action";
          /** The item's identity, which a choice reports back. */
          readonly id: string;
          /** The label. */
          readonly title: string;
          /** The key combination choosing it, such as `mod+,`. */
          readonly shortcut?: string;
      }
    | {
          /** An item with a check mark the person toggles. */
          readonly kind: "toggle";
          /** The item's identity, which a choice reports back. */
          readonly id: string;
          /** The label. */
          readonly title: string;
          /** Whether the item shows its check mark. */
          readonly isChecked: boolean;
      }
    | {
          /** A line between groups of items. */
          readonly kind: "separator";
      };

/** A command the host sends the native shell through its private input pipe. */
export type Command =
    // windows
    | {
          /** Open a window, or focus the one with the identity. */
          readonly action: "window.open";
          /** The window's identity, which later commands and events include. */
          readonly id: string;
          /** What the window shows. */
          readonly kind: WindowKind;
          /** The location, which its origin authorized. */
          readonly url: string;
          /** The other origins the window navigates within, such as the universe signing it in. */
          readonly origins: readonly string[];
          /** The browser profile whose storage the window uses. */
          readonly profile: string;
          /** The title bar's text. */
          readonly title: string;
          /** Where the window sits, the shell's choice when absent. */
          readonly bounds?: WindowBounds;
      }
    | {
          /** Bring a window into focus. */
          readonly action: "window.focus";
          /** The window's identity. */
          readonly id: string;
      }
    | {
          /** Close a window without reporting it closed. */
          readonly action: "window.close";
          /** The window's identity. */
          readonly id: string;
      }

    // banners and the badge
    | {
          /** Show a banner, replacing the one with the same identity. */
          readonly action: "banner.show";
          /** The banner's identity, which withdrawals and events include. */
          readonly id: string;
          /** The headline. */
          readonly title: string;
          /** The line below the headline. */
          readonly subtitle?: string;
          /** The text. */
          readonly body: string;
          /** How strongly the banner interrupts, which the system's focus modes follow. */
          readonly interruption: InterruptionLevel;
          /** The actions offered beside it. */
          readonly actions: readonly BannerAction[];
      }
    | {
          /** Withdraw a shown banner. */
          readonly action: "banner.withdraw";
          /** The banner's identity. */
          readonly id: string;
      }
    | {
          /** Show a count on the application icon. */
          readonly action: "badge.set";
          /** The count to show, or null to clear the badge. */
          readonly count: number | null;
      }
    | {
          /** Remove a browser profile and everything its windows stored, as signing out does. */
          readonly action: "profile.remove";
          /** The profile. */
          readonly profile: string;
      }

    // menus, the tray, shortcuts and the browser
    | {
          /** Replace the host's items in a native menu. */
          readonly action: "menu.set";
          /** The menu. */
          readonly menu: MenuName;
          /** The items, in order. */
          readonly items: readonly MenuItem[];
      }
    | {
          /** Title the shell's built-in menu items in the host's language. */
          readonly action: "menu.title";
          /** The title of the item opening a new window. */
          readonly start: string;
          /** The title of the item quitting the desktop. */
          readonly quit: string;
      }
    | {
          /** Show the desktop's status on the tray icon. */
          readonly action: "tray.set";
          /** The text the icon shows on hover. */
          readonly tooltip: string;
      }
    | {
          /** Register a global shortcut, replacing the one with the same identity. */
          readonly action: "shortcut.register";
          /** The shortcut's identity, which a press reports back. */
          readonly id: string;
          /** The key combination, such as `alt+space`. */
          readonly shortcut: string;
      }
    | {
          /** Release a global shortcut, which presses then no longer report. */
          readonly action: "shortcut.unregister";
          /** The shortcut's identity. */
          readonly id: string;
      }
    | {
          /** Open a web location in the default browser, such as a sign-in. */
          readonly action: "browser.open";
          /** The location. */
          readonly url: string;
      }

    // application
    | {
          /** Open the desktop at login, or stop opening it. */
          readonly action: "autostart.set";
          /** Whether the desktop opens at login. */
          readonly isEnabled: boolean;
      }
    | {
          /** Close every window and exit. */
          readonly action: "application.quit";
      };

/** A native observation the shell sends the host through its private output pipe. */
export const Event = schema.discriminatedUnion("action", [
    // windows
    schema.object({
        /** The person closed a window. */
        action: schema.literal("window.closed"),
        /** The window's identity. */
        id: schema.identifier("window"),
    }),
    schema.object({
        /** A window navigated off its origin, which the shell stopped. */
        action: schema.literal("window.navigated"),
        /** The window's identity. */
        id: schema.identifier("window"),
        /** The location. */
        url: schema.string().min(1),
    }),
    schema.object({
        /** The person moved or resized a window. */
        action: schema.literal("window.moved"),
        /** The window's identity. */
        id: schema.identifier("window"),
        /** Where the window sits now. */
        bounds: WindowBounds,
    }),
    schema.object({
        /** A window came into focus. */
        action: schema.literal("window.focused"),
        /** The window's identity. */
        id: schema.identifier("window"),
    }),

    // banners
    schema.object({
        /** The person chose an action beside a banner. */
        action: schema.literal("banner.acted"),
        /** The banner's identity. */
        id: schema.string().min(1),
        /** The action's declared name. */
        name: schema.string().min(1),
        /** The text the action asked for. */
        text: schema.string().exactOptional(),
    }),
    schema.object({
        /** The person dismissed a banner. */
        action: schema.literal("banner.dismissed"),
        /** The banner's identity. */
        id: schema.string().min(1),
    }),

    // links, menus and shortcuts
    schema.object({
        /** Another application opened a link, or the person asked for a new window. */
        action: schema.literal("link.opened"),
        /** The link, or null for the start link. */
        url: schema.string().min(1).nullable(),
    }),
    schema.object({
        /** The person chose a host item of a native menu. */
        action: schema.literal("menu.chosen"),
        /** The item's identity. */
        item: schema.string().min(1),
    }),
    schema.object({
        /** The person pressed a registered global shortcut. */
        action: schema.literal("shortcut.pressed"),
        /** The shortcut's identity. */
        id: schema.string().min(1),
    }),

    // application
    schema.object({
        /** The person launched the desktop again, or clicked its icon without a window showing. */
        action: schema.literal("application.reopened"),
    }),
    schema.object({
        /** A command or a native step of the shell failed. */
        action: schema.literal("shell.failed"),
        /** What failed: a command's action, such as window.open, or a native step. */
        command: schema.string().min(1),
        /** Why it failed. */
        message: schema.string(),
    }),
]);
/** A native observation the shell sends the host. */
export type Event = schema.Infer<typeof Event>;
