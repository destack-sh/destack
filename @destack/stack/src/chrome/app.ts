import { InstallationOrigin } from "@destack/account/address";
import { IconName } from "@destack/icon";
import { type Message, t } from "@destack/locale";
import { PACKAGE_KIND } from "@destack/object";
import { ObjectStoreDescription } from "@destack/object/client";
import { ViewDescription } from "@destack/package/manifest";
import { Hue } from "@destack/style";
import { VIEW_KIND } from "@destack/view/declare";

/** The icon of an app whose package presents none, as a window. */
const APP_ICON = "app-window";

/** The hue of an app whose package presents none, as neutral. */
const APP_HUE = "gray";

/** An app of a space: an installation with a main view, as its package presents itself. */
export interface SpaceApp {
    /** The installation's name in its space, which its origin names. */
    readonly name: string;
    /** The view its origin opens. */
    readonly view: string;
    /** The package's title. */
    readonly title: Message;
    /** The package's icon. */
    readonly icon: IconName;
    /** The package's hue. */
    readonly color: Hue;
    /** Whether it is the space's home, the app with a view in the home category. */
    readonly isHome: boolean;
}

/** An app of a space, read from the space's store description. */
export const SpaceApp = {
    /** List a space's apps by name, from its store's description of its packages and views. */
    async list(store: Parameters<typeof ObjectStoreDescription.read>[0]): Promise<SpaceApp[]> {
        // describe the space's packages with their views and presentations, in one call
        const description = await ObjectStoreDescription.read(store, {
            kinds: [VIEW_KIND, PACKAGE_KIND],
        });
        const apps = description.packages.flatMap((implementation): SpaceApp[] => {
            // keep the installations with a main view, leaving services out
            const name = implementation.installation?.name;
            const views = Object.fromEntries(
                description
                    .declarations(implementation, VIEW_KIND)
                    .map((view) => [view.name, view.description]),
            );
            const main = ViewDescription.main(views);
            if (name === undefined || main === undefined) {
                return [];
            }

            // present it as its package does, as a window in gray where it says nothing
            const [presented] = description.declarations(implementation, PACKAGE_KIND);
            const icon = IconName.safeParse(presented?.description.icon);
            const hue = Hue.safeParse(presented?.description.color);

            return [
                {
                    name,
                    view: main,
                    title: presented?.description.title ?? t`${name}`,
                    icon: icon.success ? icon.data : APP_ICON,
                    color: hue.success ? hue.data : APP_HUE,
                    isHome: Object.values(views).some((view) => view.categories.includes("home")),
                },
            ];
        });

        return apps.toSorted((left, right) => (left.name < right.name ? -1 : 1));
    },

    /** Write where an app opens: its main view at its installation's origin, beside a page's. */
    href(app: SpaceApp, page: URL, origin: InstallationOrigin, domain: string): string {
        const hostname = InstallationOrigin.format(
            { handle: origin.handle, space: origin.space, name: app.name },
            domain,
        );
        const port = page.port === "" ? "" : `:${page.port}`;

        return `${page.protocol}//${hostname}${port}/${app.view}`;
    },
};
