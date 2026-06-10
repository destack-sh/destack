import { Seo } from "../component/seo";
import { Shell } from "../component/shell";
import { EditorShell } from "../editor/shell";
import { createWorkspace } from "../editor/workspace";

export function HomePage() {
    const workspace = createWorkspace();

    return (
        <Shell isFixed>
            <Seo />
            <section class="mx-auto h-full min-h-0 w-full max-w-328 min-w-0 px-4 py-5 md:px-6 md:py-6">
                <EditorShell workspace={workspace} />
            </section>
        </Shell>
    );
}
