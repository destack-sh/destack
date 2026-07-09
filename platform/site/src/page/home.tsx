import { Seo } from "../component/seo";
import { Shell } from "../component/shell";
import { Compilation } from "../component/compilation";

/// The public installation command.
const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// The current Destack product outline.
const points = [
    "a universal software engine for correct, optimal, integrated software",
    "a TypeScript++ language, compiler, VM, runtime, and libraries",
    "web and native targets from one integrated toolchain",
    "simulation and introspection built into the system",
    "incrementally granular building blocks for your own software stack",
] as const;

/// The real Destack mark sampled into a fixed-width luminance field.
const planet = [
    "",
    "",
    "",
    "",
    "",
    "                          .::-------:..",
    "                     .:-+*#########****+=:.",
    "                   :=*########*************+-.",
    "                 :+############***************-........",
    "               .=############******************+*%##%###**=:",
    "              .*##############***********++++++++*######*#%%=.",
    "             .+++****************+++++++++++++++++=-==:=*:+@#:",
    "           . +******+++++++++++++++++++++++++++++++. *:=#+%%=",
    "          . :**********++++++++*****************+++==+*#%@#-",
    "            =*****+++++++**************++++++++++++*#%%%#-.",
    "          .-*********************************++**#%%@#+-.",
    "       .-+#@*+++++**********************+++***#%@@%#-.",
    "     .-#%@%#*+++++++++++++++++++++++++++**#%%@%#**+=",
    "    -#@%%*+=.+***++++++++++++++++++**#%%@@%%#*+++++: .",
    "   =%%+#=:*. .***********+++***##%%@@%%##**+++++++- .",
    "  :#@+.*=.==-:-+++******###%%%@%%%##**+++++++++++- .",
    "  .=%%**##*######%%%%%%%%%%%##**++++++++++++++++: .",
    "    :=*####%#####*#%###******+******++++++++++-. .",
    "         ......... .-+********+++++++++++++=:.  .",
    "                  .  .:-=++**++++++++++=-:.   .",
    "                    .    ..:::-----::..",
    "",
    "",
    "",
    "",
    "",
    "",
].join("\n");

/// The sampled ring separated from the bright page background.
const planetRing = [
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "                                                 %##%###**=:",
    "                                                  ######*#%%=",
    "                                                   -     :+@#:",
    "                                                     *:  +%%=",
    "                                                    =  #%@#-",
    "                                                    #%%%#-",
    "           -                                     #%%@#+-",
    "        -+#@                                  #%@@%",
    "      -#%@%#                              #%%@%",
    "    -#@%%*+=.                        #%%@@%",
    "   =%%+   *                     #%%@@%%",
    "  :#@+       :           ##%%%@%%",
    "   =%%**##*######%%%%%%%%%%",
    "    :=*####%#####*#",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
].join("\n");

/// Render the public Destack homepage.
export function HomePage() {
    return (
        <Shell>
            <Seo
                description={
                    "Destack is an experimental language, compiler, VM, and runtime for " +
                    "building complete software systems with TypeScript."
                }
            />

            <article class="home-page">
                {/* Main pitch */}
                <section class="home-hero">
                    <div class="home-hero__copy">
                        <h1>destack</h1>
                        <p class="home-hero__statement">a system for understanding systems.</p>

                        <ul class="home-points">
                            {points.map((point) => (
                                <li>{point}</li>
                            ))}
                        </ul>

                        <nav aria-label="Primary actions" class="home-actions">
                            <a
                                href={
                                    "https://github.com/destack-sh/destack/blob/main/" +
                                    "language/DESIGN.md"
                                }
                            >
                                [design]
                            </a>
                            <a href="https://github.com/destack-sh/destack">[github]</a>
                        </nav>
                    </div>

                    <AsciiPlanet />
                </section>

                {/* Compilation */}
                <Compilation />

                {/* Installation */}
                <section class="home-install" id="install">
                    <header>
                        <h2>[install]</h2>
                    </header>
                    <code>
                        <span>$</span> {installCommand}
                    </code>
                </section>
            </article>
        </Shell>
    );
}

/// Render the actual Destack mark through sampled ASCII characters.
function AsciiPlanet() {
    return (
        <figure aria-label="The Destack ringed planet" class="ascii-planet" role="img">
            <div aria-hidden="true" class="ascii-planet__art">
                <pre class="ascii-planet__surface">{planet}</pre>
                <pre class="ascii-planet__ring">{planetRing}</pre>
            </div>
            <pre aria-hidden="true" class="ascii-planet__shadow">.::================::.</pre>
        </figure>
    );
}
