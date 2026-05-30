import { Link, Meta, Title } from "@solidjs/meta";

const siteUrl = "https://destack.sh";
const siteTitle = "Destack";
const siteDescription =
    "Destack is a universal software engine for building correct, optimal, integrated software.";

type SeoProps = {
    description?: string;
    path?: string;
    title?: string;
    type?: "article" | "website";
};

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
            <Meta property="og:site_name" content={siteTitle} />
            <Meta property="og:title" content={title()} />
            <Meta property="og:type" content={type()} />
            <Meta property="og:url" content={url()} />
            <Meta name="twitter:card" content="summary" />
            <Meta name="twitter:description" content={description()} />
            <Meta name="twitter:title" content={title()} />
            <Link rel="canonical" href={url()} />
        </>
    );
}
