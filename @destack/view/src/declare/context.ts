import { DeclarationName } from "@destack/package";
import type { ImportMap } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { ViewError } from "../error/error.ts";

/** The same-origin host path opening an object, a view or the command palette. */
export const OPEN_PATH = "/.destack/open";

/** The same-origin host path below which a browser registers its client. */
export const CLIENT_PATH = "/.destack/client";

/** The same-origin host path below which a view calls the home of a scope, `<path>/<scope>`. */
export const HOME_PATH = "/.destack/home";

/** The same-origin host path below which a view calls the platform services it declares. */
export const PLATFORM_PATH = "/.destack/platform";

/** The identifier of the script element with the view's context. */
export const CONTEXT_ELEMENT = "destack-view";

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

/** The query parameter of a view's address with the access token launching it. */
export const TOKEN_PARAMETER = "token";

/** The scopes a host opens for a view, by their key in its context. */
export const CONTEXT_SCOPES = ["space", "account", "home"] as const;

/** A scope a host opens for a view. */
export type ContextScope = (typeof CONTEXT_SCOPES)[number];

/** What the host gives a view: where and for whom it runs. */
const viewContext = schema.object({
    /** The installation serving the view. */
    installation: schema.string().min(1),
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
});

/** What the host gives a view: where and for whom it runs. */
export const ViewContext = Object.assign(viewContext, {
    /** Read one of a context's scopes, refusing one the host opens none for. */
    scope(context: ViewContext, name: ContextScope): string {
        // require a scope the host opens
        const found = context[name];
        if (found === undefined) {
            throw ViewError.of("SCOPE_NOT_OPENED", `the host does not open ${name} for this view`);
        }

        return found;
    },

    /** Write the document mounting a view with its context, import map and entry. */
    document(context: ViewContext, entrypoint: string, map: ImportMap): Response {
        // escape the context and the import map for script elements
        const json = JSON.stringify(viewContext.parse(context)).replaceAll("<", "\\u003c");
        const imports = JSON.stringify(map).replaceAll("<", "\\u003c");
        const html = `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<script type="application/json" id="${CONTEXT_ELEMENT}">${json}</script>
<script type="importmap">${imports}</script>
<script type="module" src="/${entrypoint}"></script>
</head>
<body></body>
</html>
`;

        return new Response(html, {
            headers: {
                "content-type": "text/html; charset=utf-8",
                "cache-control": "no-store",
            },
        });
    },
});
/** What the host gives a view. */
export type ViewContext = schema.Infer<typeof viewContext>;
