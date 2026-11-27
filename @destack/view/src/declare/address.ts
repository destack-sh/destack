import { DeclarationName, OriginPath } from "@destack/package";
import { schema } from "@destack/schema";
import { SpaceAddress } from "@destack/account/address";
import { ObjectReference } from "@destack/sync";
import {
    INSTALLATION_PARAMETER,
    OBJECT_PARAMETER,
    PATH_PARAMETER,
    SPACE_PARAMETER,
    VIEW_PARAMETER,
} from "./context.ts";

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
    /** Write the address of the view's page at its installation's origin. */
    url(address: ViewAddress, origin: string): string {
        const path = address.path === "/" ? "" : address.path;

        return new URL(`${origin}/${address.view}${path}`).href;
    },

    /** Write the same-origin path where the host sends a page load on to the view's origin. */
    openPath(address: ViewAddress): string {
        const query = new URLSearchParams({
            [SPACE_PARAMETER]: address.space,
            [INSTALLATION_PARAMETER]: address.installation,
            [VIEW_PARAMETER]: address.view,
            [PATH_PARAMETER]: address.path,
        });

        return `${OriginPath.open}?${query.toString()}`;
    },

    /** Read the view and location an open path's query addresses. */
    safeParseOpenPath(url: URL): ReturnType<typeof viewAddress.safeParse> {
        return viewAddress.safeParse({
            space: url.searchParams.get(SPACE_PARAMETER),
            installation: url.searchParams.get(INSTALLATION_PARAMETER),
            view: url.searchParams.get(VIEW_PARAMETER),
            path: url.searchParams.get(PATH_PARAMETER),
        });
    },
});
/** A view of an installation in a space. */
export type ViewAddress = schema.Infer<typeof viewAddress>;

/** An object of a space, opened in the view presenting its type, or in the view asked for. */
const objectAddress = schema.object({
    /** The space's address. */
    space: SpaceAddress,
    /** The object. */
    object: ObjectReference,
    /** The view asked for, absent for the strongest view presenting the type. */
    view: DeclarationName.exactOptional(),
});
/** An object of a space, opened in the view presenting its type, or in the view asked for. */
export const ObjectAddress = Object.assign(objectAddress, {
    /** Write the same-origin path where the host opens the object, in the view asked for if any. */
    openPath(address: Pick<ObjectAddress, "object" | "view">): string {
        // name the object, and the view when asked for
        const query = new URLSearchParams({ [OBJECT_PARAMETER]: JSON.stringify(address.object) });
        if (address.view !== undefined) {
            query.set(VIEW_PARAMETER, address.view);
        }

        return `${OriginPath.open}?${query.toString()}`;
    },
});
/** An object of a space, opened in the view presenting its type, or in the view asked for. */
export type ObjectAddress = schema.Infer<typeof objectAddress>;

/** What a launch opens: a view of an installation, or an object of a space. */
export type Address = ViewAddress | ObjectAddress;
