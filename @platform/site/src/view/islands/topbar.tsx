import { TopBar } from "../layout/topbar";

/** Render the site navigation for the page at a path. */
export default function TopBarIsland(properties: { readonly path: string }) {
    return <TopBar path={properties.path} />;
}
