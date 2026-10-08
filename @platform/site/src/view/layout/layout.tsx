import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Loading } from "@destack/view";
import { type RouteSectionProps, useNavigate } from "@destack/view/router";
import { Accelerator, isCommandPlatform } from "@destack/view/palette";
import { createShortcut } from "@destack/view/primitives/keyboard";
import { defineMetadata, Metadata } from "@destack/view/document";

import { origin, tagline } from "../content/site";
import { GooField } from "../effect/goo";
import { Footer } from "./footer";
import { destinations, isExternal } from "./navigation";
import { TopBar } from "./topbar";

/** The metadata every page starts from: the site's name, origin, preview and icons. */
const metadata = defineMetadata({
    metadataBase: origin,
    title: { template: "%s | Destack", default: "Destack" },
    description: tagline,
    applicationName: "Destack",
    openGraph: { type: "website", siteName: "Destack", images: ["/og.png"] },
    twitter: { card: "summary_large_image" },
    icons: {
        icon: [
            { url: "/brand/favicon/favicon.svg", type: "image/svg+xml" },
            { url: "/brand/favicon/favicon-32.png", type: "image/png", sizes: "32x32" },
        ],
        apple: "/brand/icon/icon-180.png",
    },
    manifest: "/manifest.webmanifest",
});

/** Lay out every page in the site frame, with the site's one goo field behind its goo cells. */
export function Layout(properties: RouteSectionProps) {
    // open each destination on its key combination, leaving the site in a new tab
    const navigate = useNavigate();
    for (const destination of destinations) {
        createShortcut(
            Accelerator.keys(destination.keybinding, isCommandPlatform()),
            () => {
                if (isExternal(destination)) {
                    window.open(destination.href, "_blank", "noopener");
                } else {
                    navigate(destination.href);
                }
            },
            { preventDefault: true, ignoreWithinInputs: true, anyOrder: true },
        );
    }

    return (
        <Metadata {...metadata}>
            <div {...style.attrs(styles.root)}>
                <GooField />
                <TopBar />
                <main {...style.attrs(styles.main)}>
                    <Loading>{properties.children}</Loading>
                </main>
                <Footer />
            </div>
        </Metadata>
    );
}

/** The site frame styles. */
const styles = style.create({
    root: {
        color: color.foreground,
        display: "grid",
        fontFamily: font.text,
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        isolation: "isolate",
        minHeight: "100svh",
        overflow: "clip",
        position: "relative",
    },
    main: {
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
    },
});
