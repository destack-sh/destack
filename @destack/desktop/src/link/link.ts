import type { Stage } from "@destack/update/release";
import { InstallationOrigin, SpaceAddress, SpaceOrigin } from "@destack/account/address";
import {
    OBJECT_PARAMETER,
    ObjectAddress,
    OPEN_PATH,
    type Address,
    TOKEN_PARAMETER,
    VIEW_PARAMETER,
    ViewAddress,
    objectUrl,
} from "@destack/view/declare";

/** The deep links to views and objects under a stage's desktop scheme. */
export class Link {
    /** The URL scheme the system routes to the stage's desktop, which is the stage's command. */
    readonly scheme: string;

    /** Write and read links under a scheme. */
    constructor(scheme: string) {
        this.scheme = scheme;
    }

    /** Write and read the links of a stage's desktop. */
    static of(stage: Stage): Link {
        return new Link(stage.command);
    }

    /** Read the view or object of a link, or nothing for neither. */
    parse(href: string): Address | undefined {
        // require this stage's scheme without a port or credentials
        if (!URL.canParse(href)) {
            return undefined;
        }
        const url = new URL(href);
        if (url.protocol !== `${this.scheme}:` || url.port || url.username || url.password) {
            return undefined;
        }

        // read a view of the host's installation
        const space = SpaceOrigin.parse(url.hostname);
        if (space === undefined) {
            return Link.view(url);
        }
        // read an object at the open path of the host's space, never of a branch
        else if (url.pathname === OPEN_PATH && space.branch === undefined) {
            return Link.object(url, SpaceAddress.of(space));
        }
        // refuse any other path of a space
        else {
            return undefined;
        }
    }

    /** Read the name an address shows under: its space for an object, else its installation. */
    static title(address: Address): string {
        return "object" in address ? address.space : address.installation;
    }

    /** Read the view a location shows on its installation's origin below a domain. */
    static view(url: URL, domain?: string): ViewAddress | undefined {
        // read the host's installation
        const origin = InstallationOrigin.parse(url.hostname, domain);
        if (origin === undefined || origin.branch !== undefined || origin.digest !== undefined) {
            return undefined;
        }

        // split the view from its location, dropping the launch token
        const [, view, ...rest] = url.pathname.split("/");
        const search = new URLSearchParams(url.search);
        search.delete(TOKEN_PARAMETER);
        const query = search.size === 0 ? "" : `?${search.toString()}`;
        const address = ViewAddress.safeParse({
            space: SpaceAddress.of(origin),
            installation: origin.name,
            view,
            path: `/${rest.join("/")}${query}`,
        });

        return address.success ? address.data : undefined;
    }

    /** Read the view a location shows on an origin beside another: its scheme, port and domain. */
    static beside(origin: URL, url: URL): Address | undefined {
        // read the domain below the origin's label, requiring a label, its scheme and its port
        const dot = origin.hostname.indexOf(".");
        const isBeside = dot !== -1 && url.protocol === origin.protocol && url.port === origin.port;

        return isBeside ? Link.view(url, origin.hostname.slice(dot + 1)) : undefined;
    }

    /** Read the object and the view of an open link's query in a space. */
    static object(url: URL, space: string): ObjectAddress | undefined {
        // parse the object's JSON, refusing text that is none
        const object = url.searchParams.get(OBJECT_PARAMETER);
        let reference: unknown;
        try {
            reference = object === null ? undefined : JSON.parse(object);
        } catch {
            return undefined;
        }

        // require the object, and a view when one is asked for
        const view = url.searchParams.get(VIEW_PARAMETER);
        const address = ObjectAddress.safeParse({
            space,
            object: reference,
            ...(view === null ? {} : { view }),
        });

        return address.success ? address.data : undefined;
    }

    /** Write the link to a view, leaving out the view's root location, or an object. */
    format(address: Address): string {
        // link the object at the open path of its space's origin without a domain
        if ("object" in address) {
            const options = address.view === undefined ? {} : { view: address.view };

            return `${this.scheme}://${address.space}${objectUrl(address.object, options)}`;
        }

        // link the view below its installation's origin without a domain
        const space = SpaceAddress.origin(address.space);
        const origin = InstallationOrigin.format({ ...space, name: address.installation });
        const view = `${this.scheme}://${origin}/${address.view}`;

        return address.path === "/" ? view : `${view}${address.path}`;
    }
}
