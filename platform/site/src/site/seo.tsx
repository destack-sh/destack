import { Link, Meta, Title } from "@destack/view/document";

/// The canonical public site origin.
const siteUrl = "https://destack.sh";

/// The site name used in metadata titles.
const siteTitle = "Destack";

/// The default search and social description.
const siteDescription =
    "Destack is a universal software engine: one language, one toolchain, and one runtime for libraries, services, and apps, compiled to native and the web.";

type SeoProps = {
    /// The page summary used by search and social previews.
    description?: string;

    /// The public Markdown representation of this page.
    markdownRoute?: string;

    /// The canonical site path.
    path?: string;

    /// The public plain-text representation of this page.
    textRoute?: string;

    /// The page title without the site suffix.
    title?: string;

    /// The Open Graph page type.
    type?: "article" | "website";
};

/// Render canonical, search, and social metadata for one page.
export function Seo(props: SeoProps) {
    const title = () => (props.title == undefined ? siteTitle : `${props.title} | ${siteTitle}`);
    const description = () => props.description ?? siteDescription;
    const type = () => props.type ?? "website";
    const url = () => `${siteUrl}${props.path ?? "/"}`;

    return (
        <>
            <Title>{title()}</Title>
            <Meta name="description" content={description()} />
            <Meta property="og:description" content={description()} />
            <Meta property="og:image" content={`${siteUrl}/og.png`} />
            <Meta property="og:site_name" content={siteTitle} />
            <Meta property="og:title" content={title()} />
            <Meta property="og:type" content={type()} />
            <Meta property="og:url" content={url()} />
            <Meta name="twitter:card" content="summary_large_image" />
            <Meta name="twitter:description" content={description()} />
            <Meta name="twitter:image" content={`${siteUrl}/og.png`} />
            <Meta name="twitter:title" content={title()} />
            <Link rel="canonical" href={url()} />
            {props.markdownRoute != undefined && (
                <Link
                    rel="alternate"
                    type="text/markdown"
                    href={`${siteUrl}${props.markdownRoute}`}
                />
            )}
            {props.textRoute != undefined && (
                <Link rel="alternate" type="text/plain" href={`${siteUrl}${props.textRoute}`} />
            )}
        </>
    );
}
