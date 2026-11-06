import { color, font } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import { Loading } from "@destack/view";
import { type RouteSectionProps, useLocation } from "@destack/view/router";
import { defineMetadata, Metadata } from "@destack/view/document";

import { brand } from "../brand";
import { origin, tagline } from "../content/site";
import Shortcuts from "../islands/shortcuts";
import TopBarIsland from "../islands/topbar";
import { Footer } from "./footer";
import { paper } from "./lattice.stylex";

/** The metadata every page starts from: the site's name, origin, preview and icons. */
const metadata = defineMetadata({
    metadataBase: origin,
    title: { template: "%s | Destack", default: "Destack" },
    description: tagline,
    applicationName: "Destack",
    openGraph: { type: "website", siteName: "Destack", images: [brand.banner] },
    twitter: { card: "summary_large_image" },
    icons: {
        icon: [
            { url: brand.favicon, type: "image/svg+xml" },
            { url: brand.favicon32, type: "image/png", sizes: "32x32" },
        ],
        apple: brand.icon180,
    },
    manifest: "/manifest.webmanifest",
});

/** Lay out every page in the site frame, between the sky bands of its top bar and footer. */
export function Layout(properties: RouteSectionProps) {
    // follow the route the page renders at, which the navigation marks
    const location = useLocation();

    return (
        <Metadata {...metadata}>
            <div {...style.attrs(paper.ground, styles.root)}>
                <Shortcuts />
                <TopBarIsland path={location.pathname} />
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
