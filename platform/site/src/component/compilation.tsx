import { destackHtml, javascriptHtml, mirHtml } from "../generated/compilation";

/// Render one illustrative Destack compilation.
export function Compilation() {
    return (
        <section class="home-compile">
            <header class="home-compile__header">
                <h2>[compile]</h2>
                <span>illustrative output, compiler generated soon</span>
            </header>

            <div class="home-compile__body">
                <Listing html={destackHtml} name="main.ds" />

                <pre aria-hidden="true" class="home-compile__branch">{`--+-->
  |
  +-->`}</pre>

                <div class="home-compile__outputs">
                    <Listing html={mirHtml} name="main.mir" />
                    <Listing html={javascriptHtml} name="main.js" />
                </div>
            </div>
        </section>
    );
}

/// Properties for one highlighted source listing.
type ListingProps = {
    /// The highlighted source HTML.
    html: string;
    /// The displayed source name.
    name: string;
};

/// Render one framed source artifact.
function Listing(props: ListingProps) {
    return (
        <figure class="home-code">
            <figcaption>[{props.name}]</figcaption>
            <pre>
                <code innerHTML={props.html} />
            </pre>
        </figure>
    );
}
