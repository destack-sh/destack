import { graph } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";

/** The extensions of the script files a fixture graph describes as modules. */
const SCRIPT = /\.(?:[cm]?[jt]s|tsx|jsx)$/u;

/** Describe a fixture build's graph: each script a module with its declarations. */
export async function fixtureGraph(reader: BuildReader): Promise<graph.Module[]> {
    // group the build's declarations by the module declaring them
    const declarations = Map.groupBy(await reader.declarations(), (each) =>
        graph.Moniker.module(each.symbol),
    );

    // describe each script the build holds with the declarations it makes
    const scripts = (await reader.files()).filter((file) => SCRIPT.test(file.path));

    return scripts.map((file) =>
        graph.Module.of({
            path: file.path,
            digest: file.digest,
            imports: [],
            exports: [],
            declarations: declarations.get(file.path) ?? [],
            symbols: [],
            edges: [],
        }),
    );
}
