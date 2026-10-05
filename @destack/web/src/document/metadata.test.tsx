import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import { defineMetadata, defineViewport, Metadata, Viewport } from "./metadata.tsx";

/** A site's metadata defaults, as its layout declares them. */
const site = defineMetadata({
    metadataBase: "https://destack.sh",
    title: { template: "%s | Destack", default: "Destack" },
    description: "Software you can see into.",
    openGraph: {
        siteName: "Destack",
        images: [{ url: "/og.png", width: 1200, height: 630, alt: "The Destack lattice" }],
    },
    twitter: { card: "summary_large_image" },
    icons: {
        icon: [{ url: "/brand/favicon/favicon.svg", type: "image/svg+xml" }],
        apple: "/brand/icon/icon-180.png",
    },
    manifest: "/manifest.webmanifest",
    verification: { google: "token-1" },
});

/** Render metadata into the document head, removing it after the test. */
async function mount(page: () => ReturnType<typeof Metadata>): Promise<void> {
    const host = document.createElement("main");
    document.body.append(host);
    const dispose = render(page, host);
    onTestFinished(() => {
        dispose();
        host.remove();
        document.head.replaceChildren();
    });

    // wait for the head registry, which applies registered tags after the current task
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

/** Read the content of the head's named or property metadata. */
function meta(key: string): string | null | undefined {
    return document.head
        .querySelector(`meta[name="${key}"], meta[property="${key}"]`)
        ?.getAttribute("content");
}

/** Read the head's title, canonical address and named and property metadata. */
function head() {
    return {
        title: document.title,
        canonical: document.head.querySelector('link[rel="canonical"]')?.getAttribute("href"),
        markdown: document.head.querySelector('link[type="text/markdown"]')?.getAttribute("href"),
        description: meta("description"),
        ogTitle: meta("og:title"),
        ogType: meta("og:type"),
        ogUrl: meta("og:url"),
        ogImage: meta("og:image"),
        ogImageWidth: meta("og:image:width"),
        twitterCard: meta("twitter:card"),
        icon: document.head.querySelector('link[rel="icon"]')?.getAttribute("type"),
        manifest: document.head.querySelector('link[rel="manifest"]')?.getAttribute("href"),
        verification: meta("google-site-verification"),
    };
}

test("fill the site's title template and resolve addresses against its base for a page", async () => {
    // render a post's metadata inside the site's
    await mount(() => (
        <Metadata {...site}>
            <Metadata
                title="Seeing software"
                description="Why Destack shows you the program."
                alternates={{
                    canonical: "/blog/seeing",
                    types: { "text/markdown": "/blog/seeing.md" },
                }}
                openGraph={{ type: "article" }}
            />
        </Metadata>
    ));

    // inherit the site's image, icon and card under the page's own title, description and type
    expect(head()).toEqual({
        title: "Seeing software | Destack",
        canonical: "https://destack.sh/blog/seeing",
        markdown: "https://destack.sh/blog/seeing.md",
        description: "Why Destack shows you the program.",
        ogTitle: "Seeing software | Destack",
        ogType: "article",
        ogUrl: "https://destack.sh/blog/seeing",
        ogImage: "https://destack.sh/og.png",
        ogImageWidth: "1200",
        twitterCard: "summary_large_image",
        icon: "image/svg+xml",
        manifest: "/manifest.webmanifest",
        verification: "token-1",
    });
});

test("show the site's default title and preview for a page naming none", async () => {
    // render the site's metadata alone
    await mount(() => <Metadata {...site} />);

    // keep the default title, description and the image against the base
    expect(head()).toMatchObject({
        title: "Destack",
        description: "Software you can see into.",
        ogImage: "https://destack.sh/og.png",
    });
});

test("write the viewport, the browser interface colors per scheme and the color schemes", async () => {
    // render a viewport with a light and a dark theme color
    const viewport = defineViewport({
        themeColor: [
            { media: "(prefers-color-scheme: light)", color: "#ffffff" },
            { media: "(prefers-color-scheme: dark)", color: "#0b0b0f" },
        ],
        colorScheme: "light dark",
    });
    await mount(() => <Viewport {...viewport} />);

    // write the default layout, each scheme's color and the schemes
    expect({
        viewport: meta("viewport"),
        colors: [...document.head.querySelectorAll('meta[name="theme-color"]')].map((color) => [
            color.getAttribute("media"),
            color.getAttribute("content"),
        ]),
        scheme: meta("color-scheme"),
    }).toEqual({
        viewport: "width=device-width, initial-scale=1",
        colors: [
            ["(prefers-color-scheme: light)", "#ffffff"],
            ["(prefers-color-scheme: dark)", "#0b0b0f"],
        ],
        scheme: "light dark",
    });
});
