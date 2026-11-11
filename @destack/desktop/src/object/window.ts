import { through } from "@destack/access";
import type { Select } from "@destack/db";
import { defineObject, device, field } from "@destack/object";
import { schema } from "@destack/schema";
import { controlled } from "@destack/object";
import { argument, plural, t } from "@destack/locale";

/** A window's place on its screen, in logical pixels from the screen's top left corner. */
export const WindowBounds = schema.object({
    /** The left edge. */
    x: schema.number(),
    /** The top edge. */
    y: schema.number(),
    /** The width. */
    width: schema.number().positive(),
    /** The height. */
    height: schema.number().positive(),
});
/** A window's place on its screen. */
export type WindowBounds = schema.Infer<typeof WindowBounds>;

/** A native window of this computer: a view or an object's, or the command palette. */
export const window = defineObject(
    {
        name: "window",
        plural: "windows",
        title: t`${plural(argument("count"), { one: "Window", other: "Windows" })}`,
        scope: device,
        storage: "local",
        fields: {
            /** The link the window shows: a view, or an object opened in the view presenting it. */
            link: field.string(schema.string().min(1)),
            /** Whether it is a view's window, or the command palette floating over the others. */
            kind: field.enum(["view", "palette"]).default("view"),
            /** Where the window sits on its screen, absent until the desktop places it. */
            bounds: field.json(WindowBounds).optional(),
            /** How often the window was asked into focus, which the desktop answers once each. */
            focuses: field.integer().default(0),

            // observed
            /** The title the desktop shows, absent until it shows the window. */
            title: field.string().optional(),
            /** The browser profile isolating the storage of the origin the window shows. */
            profile: field.string(schema.uuid()).optional(),
        },
        access: {
            // let the people signed in on this computer read, open, change and close its windows
            permissions: {
                read: through("device", "use"),
                open: through("device", "use"),
                update: through("device", "use"),
                close: through("device", "use"),
            },
        },
        traits: [
            // let the desktop showing the windows observe them and remove the closed ones
            controlled(),
        ],
    },
    (method) => ({
        /** Open a window at a link, where its bounds place it, a view's unless a palette. */
        open: method.create("open", { fields: ["link", "kind", "bounds"] }),
        /** Ask the desktop to bring the window into focus. */
        focus: method.mutation({ title: t`Focus`, permission: "update" }),
        /** Move or resize the window. */
        place: method.update("update", { fields: ["bounds"] }),
        /** Close the window. */
        close: method.delete("close"),
    }),
).implement({
    methods: {
        focus: (call) => call.update({ focuses: call.target.focuses + 1 }),
    },
});
/** A window as its table stores it. */
export type WindowRow = Select<typeof window.table>;

/** The local object types the desktop stores on the device beside the machine's types. */
export const DESKTOP_OBJECTS = { window };
