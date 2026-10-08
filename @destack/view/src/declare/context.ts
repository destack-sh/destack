import { DeclarationName } from "@destack/package";
import { Catalog, LocaleTag } from "@destack/locale";
import { defineSchema, schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";

/** The same-origin host path opening an object in the view presenting it. */
export const OPEN_PATH = "/.destack/open";

/** The same-origin host path serving a view's catalogs by their content's digest. */
export const CATALOG_PATH = "/.destack/catalog";

/** The same-origin host path streaming a view's display as server-sent events. */
export const DISPLAY_PATH = "/.destack/display";

/** The same-origin host path below which a browser registers its client and binds its view credential to it. */
export const CLIENT_PATH = "/.destack/client";

/** The same-origin host path below which a view calls the platform services it declares, each at its service mount. */
export const PLATFORM_PATH = "/.destack/platform";

/** The same-origin host path of the command palette on a space's origin, and below which it reaches the space's commands. */
export const COMMAND_PATH = "/.destack/command";

/** The header the command palette's page sends its page token in. */
export const PAGE_TOKEN_HEADER = "x-csrf-token";

/** The name of the meta element carrying the command palette's page token in its document. */
export const PAGE_TOKEN_META = "csrf-token";

/** The query parameter of the command palette's path with the link of the window it opens over. */
export const FOCUS_PARAMETER = "focus";

/** The query parameter of the command palette's path with the command to run right away, as `<package>/<name>`. */
export const COMMAND_PARAMETER = "command";

/** The query parameter of the open path with the object to open, as JSON. */
export const OBJECT_PARAMETER = "object";

/** The query parameter of the open path with the view asked for. */
export const VIEW_PARAMETER = "view";

/** The query parameter of a view's address with the object it opens for, as JSON. */
export const TARGET_PARAMETER = "target";

/** The query parameter with the one-time code a view origin redeems for its credential. */
export const LAUNCH_PARAMETER = "launch";

/** A path every browser resolves on the page's own origin. */
const SameOriginPath = schema.string().regex(/^\/(?!\/)[^\t\n\r\\]*$/u);

/** What the host gives a view: where and for whom it runs. */
export const ViewContext = defineSchema(
    schema.object({
        /** The installation serving the view, absent for the shell's own pages such as the command palette. */
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
        /** The object the view opens for, from its address. */
        target: ObjectReference.exactOptional(),
        /** The language and region the person reads, the source language when the host names none. */
        locale: LocaleTag.exactOptional(),
    }),
);
/** What the host gives a view. */
export type ViewContext = schema.Infer<typeof ViewContext>;

/** A catalog a view's launch names. */
export const CatalogReference = defineSchema(
    schema.object({
        /** The package whose messages it translates. */
        package: Catalog.shape.package,
        /** The language it translates into. */
        locale: LocaleTag,
        /** The same-origin path serving it. */
        url: SameOriginPath,
    }),
);
/** A catalog a view's launch names. */
export type CatalogReference = schema.Infer<typeof CatalogReference>;

/** What the host shows a view in, pushed again whenever the person changes it. */
export const ViewDisplay = defineSchema(
    schema.object({
        /** The language and region the person reads. */
        locale: LocaleTag,
        /** The catalogs along the locale's fallback chain. */
        catalogs: schema.array(CatalogReference),
        /** The theme's custom properties and color scheme for the person's display. */
        style: schema.record(schema.string(), schema.string()),
    }),
);
/** What the host shows a view in. */
export type ViewDisplay = schema.Infer<typeof ViewDisplay>;

/** What the host embeds in a view's page as JSON in the `destack-view` script element. */
export const ViewLaunch = defineSchema(
    ViewContext.extend({
        /** The release of the installation's package serving the view. */
        release: schema.string().min(1),
        /** The same-origin path answering the replica procedures of every scope. */
        endpoint: SameOriginPath,
        /** The catalogs translating the messages of the view's package and its dependencies along the person's locale's fallback chain. */
        catalogs: schema.array(CatalogReference),
    }),
);
/** What the host embeds in a view's page. */
export type ViewLaunch = schema.Infer<typeof ViewLaunch>;
