import { DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";
import { SpaceAddress } from "@destack/account/address";
import { ObjectReference } from "@destack/sync";
import { OBJECT_PARAMETER, OPEN_PATH, TOKEN_PARAMETER, VIEW_PARAMETER } from "./context.ts";

/** A location within a view, with its query, excluding another origin. */
export const ViewPath = schema.string().regex(/^\/(?:[^/\\\r\n][^\\\r\n]*)?$/u);
/** A location within a view. */
export type ViewPath = schema.Infer<typeof ViewPath>;

/** The schema of a view of an installation in a space. */
const viewAddress = schema.object({
    /** The space's address. */
    space: SpaceAddress,
    /** The installation's name within the space. */
    installation: DeclarationName,
    /** The view the installed package declares. */
    view: DeclarationName,
    /** The location within the view. */
    path: ViewPath,
});
/** A view of an installation in a space. */
export const ViewAddress = Object.assign(viewAddress, {
    /** Write the address launching the view at its installation's origin with a token. */
    url(address: ViewAddress, origin: string, token: string): string {
        // place the view's path below the origin with the token
        const path = address.path === "/" ? "" : address.path;
        const url = new URL(`${origin}/${address.view}${path}`);
        url.searchParams.set(TOKEN_PARAMETER, token);

        return url.href;
    },
});
/** A view of an installation in a space. */
export type ViewAddress = schema.Infer<typeof viewAddress>;

/** An object of a space, opened in the view presenting its type, or in the view asked for. */
export const ObjectAddress = schema.object({
    /** The space's address. */
    space: SpaceAddress,
    /** The object. */
    object: ObjectReference,
    /** The view asked for, absent for the strongest view presenting the type. */
    view: DeclarationName.exactOptional(),
});
/** An object of a space, opened in the view presenting its type, or in the view asked for. */
export type ObjectAddress = schema.Infer<typeof ObjectAddress>;

/** What a launch opens: a view of an installation, or an object of a space. */
export type Address = ViewAddress | ObjectAddress;

/** Write the same-origin address where the host opens an object. */
export function objectUrl(
    reference: ObjectReference,
    options: { readonly view?: string } = {},
): string {
    // name the object, and the view when asked for
    const query = new URLSearchParams({ [OBJECT_PARAMETER]: JSON.stringify(reference) });
    if (options.view !== undefined) {
        query.set(VIEW_PARAMETER, options.view);
    }

    return `${OPEN_PATH}?${query.toString()}`;
}
