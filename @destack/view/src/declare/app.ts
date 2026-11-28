import { InstallationOrigin, SpaceAddress } from "@destack/account/address";
import { IconName } from "@destack/icon";
import { Message } from "@destack/locale";
import { PACKAGE_KIND } from "@destack/object";
import { DeclarationName } from "@destack/package";
import { type BuildReader, ViewDescription } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { Hue } from "@destack/style";
import { VIEW_KIND } from "./view.ts";

/** An app of the space: an installation with a main view, as its package presents itself. */
const viewApp = schema.object({
    /** The installation's name in its space, which its origin names. */
    name: schema.string().min(1),
    /** The view its origin opens. */
    view: DeclarationName,
    /** The package's title. */
    title: Message,
    /** The package's icon. */
    icon: IconName,
    /** The package's hue. */
    color: Hue,
    /** Whether it is the space's home: it has a view in the home category. */
    isHome: schema.boolean(),
});

/** An app of the space: an installation with a main view, as its package presents itself. */
export const ViewApp = Object.assign(viewApp, {
    /** Read the app an installation's build makes, none for a package without a main view. */
    async of(name: string, build: BuildReader): Promise<ViewApp | undefined> {
        // read the build's views and its package's presentation
        const views = Object.fromEntries(
            (await VIEW_KIND.read(build)).map((view) => [view.name, view.description]),
        );
        const main = ViewDescription.main(views);
        const [presented] = await PACKAGE_KIND.read(build);

        // leave out a service, which has no main view or presents no package
        if (main === undefined || presented === undefined) {
            return undefined;
        }

        return {
            name,
            view: main,
            title: presented.description.title,
            icon: presented.description.icon,
            color: presented.description.color,
            isHome: Object.values(views).some((view) => view.categories.includes("home")),
        };
    },

    /** Write where an app opens: its main view at its installation's origin, beside a page's. */
    href(app: ViewApp, spaceAddress: SpaceAddress, page: URL): string {
        // take the page's domain below its own installation label
        const [, ...domain] = page.hostname.split(".");
        const hostname = InstallationOrigin.format(
            { ...SpaceAddress.origin(spaceAddress), name: app.name },
            domain.join("."),
        );
        const port = page.port === "" ? "" : `:${page.port}`;

        return `${page.protocol}//${hostname}${port}/${app.view}`;
    },
});
/** An app of the space. */
export type ViewApp = schema.Infer<typeof viewApp>;
