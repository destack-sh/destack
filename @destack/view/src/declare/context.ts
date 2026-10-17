import { DeclarationName } from "@destack/package";
import { Catalog, LocaleTag } from "@destack/locale";
import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { Accelerator } from "../palette/accelerator.ts";
import { CommandReference } from "./command.ts";

/** The same-origin host path opening an object, a view or the command palette. */
export const OPEN_PATH = "/.destack/open";

/** The same-origin host path serving a view's catalogs by their content's digest. */
export const CATALOG_PATH = "/.destack/catalog";

/** The same-origin host path streaming a view's display as server-sent events. */
export const DISPLAY_PATH = "/.destack/display";

/** The same-origin host path below which a browser registers its client. */
export const CLIENT_PATH = "/.destack/client";

/** The same-origin host path below which a view calls the home of a scope, `<path>/<scope>`. */
export const HOME_PATH = "/.destack/home";

/** The same-origin host path below which a view calls the platform services it declares. */
export const PLATFORM_PATH = "/.destack/platform";

/** The same-origin host path of the command palette on a space's origin. */
export const COMMAND_PATH = "/.destack/command";

/** The header the command palette's page sends its page token in. */
export const PAGE_TOKEN_HEADER = "Destack-Page-Token";

/** The name of the meta element with the command palette's page token in its document. */
export const PAGE_TOKEN_META = "destack-page-token";

/** The query parameter of the command palette's path with the link of the window it opens over. */
export const FOCUS_PARAMETER = "focus";

/** The query parameter of the command palette's path with the command to run right away. */
export const COMMAND_PARAMETER = "command";

/** The query parameter of the open path with the object to open, as JSON. */
export const OBJECT_PARAMETER = "object";

/** The query parameter of the open path with the view asked for. */
export const VIEW_PARAMETER = "view";

/** The query parameter of the open path with the installation of the view asked for, by name. */
export const INSTALLATION_PARAMETER = "installation";

/** The query parameter of the open path with the address of the space of the view asked for. */
export const SPACE_PARAMETER = "space";

/** The query parameter of a view's address with the object it opens for, as JSON. */
export const TARGET_PARAMETER = "target";

/** The query parameter with the one-time code a view origin redeems for its credential. */
export const LAUNCH_PARAMETER = "launch";

/** A path every browser resolves on the page's own origin. */
const SameOriginPath = schema.string().regex(/^\/(?:[^/\t\n\r\\][^\t\n\r\\]*)?$/u);

/** The scopes a host opens for a view, by their key in its context. */
export const CONTEXT_SCOPES = ["space", "account", "home"] as const;

/** A scope a host opens for a view. */
export type ContextScope = (typeof CONTEXT_SCOPES)[number];

/** What the host gives a view: where and for whom it runs. */
export const ViewContext = Object.assign(
    schema.object({
        /** The installation serving the view, absent for the machine's command palette. */
        installation: schema.string().min(1).exactOptional(),
        /** The space of the installation. */
        space: schema.string().min(1),
        /** The account of the space. */
        account: schema.string().min(1),
        /** The declared view's name. */
        view: DeclarationName,
        /** The person using the view. */
        user: Subject,
        /** The person's home space, absent where the host opens none for the view. */
        home: schema.string().min(1).exactOptional(),
        /** The computer's device whose local objects the page reaches, absent for none. */
        device: schema.string().min(1).exactOptional(),
        /** The object the view opens for, from its address. */
        target: ObjectReference.exactOptional(),
        /** The language and region the person reads. */
        locale: LocaleTag.exactOptional(),
    }),
    {
        /** Read one of a context's scopes, refusing one the host opens none for. */
        scope(context: ViewContext, name: ContextScope): string {
            // require a scope the host opens
            const found = context[name];
            if (found === undefined) {
                throw new TypeError(`the host does not open ${name} for this view`);
            }

            return found;
        },
    },
);
/** What the host gives a view. */
export type ViewContext = schema.Infer<typeof ViewContext>;

/** A catalog a view's launch lists. */
export const CatalogReference = schema.object({
    /** The package whose messages it translates. */
    package: Catalog.shape.package,
    /** The language it translates into. */
    locale: LocaleTag,
    /** The same-origin path serving it. */
    url: SameOriginPath,
});
/** A catalog a view's launch lists. */
export type CatalogReference = schema.Infer<typeof CatalogReference>;

/** What the host shows a view in, pushed again whenever the person changes it. */
export const ViewDisplay = schema.object({
    /** The language and region the person reads. */
    locale: LocaleTag,
    /** The catalogs along the locale's fallback chain. */
    catalogs: schema.array(CatalogReference),
    /** The theme's custom properties and color scheme for the person's display. */
    style: schema.record(schema.string(), schema.string()),
    /** The key combinations running the commands of the view's installation, for the person. */
    keybindings: schema.array(
        schema.object({
            /** The command, as `<package>/<name>`. */
            command: CommandReference.key,
            /** Its key combination. */
            keybinding: Accelerator,
        }),
    ),
});
/** What the host shows a view in. */
export type ViewDisplay = schema.Infer<typeof ViewDisplay>;

/** What the host embeds in a view's page as JSON in the `destack-view` script element. */
export const ViewLaunch = ViewContext.extend({
    /** The release of the installation's package serving the view. */
    release: schema.string().min(1),
    /** The same-origin path answering the replica procedures of every scope. */
    endpoint: SameOriginPath,
    /** The catalogs that translate the messages of the view's package and its dependencies. */
    catalogs: schema.array(CatalogReference),
});
/** What the host embeds in a view's page. */
export type ViewLaunch = schema.Infer<typeof ViewLaunch>;
