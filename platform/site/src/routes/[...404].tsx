import { MissingPage } from "../site/missing";
import { Shell } from "../site/shell";

/** Render the missing page route. */
export default function NotFound() {
    return (
        <Shell>
            <MissingPage
                backHref="/"
                backLabel="back to destack"
                description="This page does not exist."
                label="404"
                title="this page does not exist"
            />
        </Shell>
    );
}
