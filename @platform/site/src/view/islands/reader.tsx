import { onSettled, useEnvironment, useTelemetry } from "@destack/view";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { renderDiagrams } from "../reader/diagram";
import { playVideo } from "../reader/media";
import { scrollAfterLayout } from "../reader/reader";

/** Draw the article's diagrams, play its videos and scroll to the heading the address has. */
export default function ReaderBehavior() {
    // read the page's telemetry and environment
    const telemetry = useTelemetry();
    const environment = useEnvironment();

    // draw the diagrams and play videos once the article is in place
    onSettled(() => {
        // find the article
        const article = document.querySelector<HTMLElement>("article[data-page-source]");
        if (article === null) {
            throw new TypeError("the reader rendered without its article");
        }

        // play its videos and draw its diagrams until the island goes
        article.addEventListener("click", playVideo);
        const stop = renderDiagrams(article, telemetry, environment);

        return () => {
            article.removeEventListener("click", playVideo);
            stop();
        };
    });

    // align direct links once the webfonts load and on each hash change
    onSettled(() => {
        void document.fonts.ready.then(() => scrollAfterLayout(environment));
    });
    createEventListener(
        () => window,
        "hashchange",
        () => scrollAfterLayout(environment),
    );

    return null;
}
