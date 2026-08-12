import { A } from "@solidjs/router";
import * as stylex from "@stylexjs/stylex";

import { commandEvents } from "../command/command";
import {
    type HomeExample,
    homeExamples,
    homeLanguageLabels,
    installCommand,
} from "../content/site";
import { homeHighlights } from "../generated/home-highlights";
import { SiteLink } from "../navigation/link";
import { socialLinks } from "../navigation/navigation";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { tokens } from "../style/tokens.stylex";
import {
    chapterStyles,
    coverStyles,
    installationStyles,
    pitchStyles,
    plateStyles,
    universeStyles,
} from "./home.stylex";

/// Properties supplied to one numbered product chapter.
type ChapterProps = {
    /// The example shown by this chapter.
    example: HomeExample;

    /// The zero-based chapter index.
    index: number;
};

/// Render the public Destack homepage.
export function HomePage() {
    return (
        <Shell isHome>
            <Seo
                description={
                    "Destack is a universal software engine for building complete " +
                    "software systems."
                }
            />

            <article {...stylex.attrs(chapterStyles.world)}>
                <Universe />
                <Cover />

                {/* System chapters */}
                <div aria-label="Destack system lifecycle" {...stylex.attrs(chapterStyles.lifecycle)}>
                    <div aria-hidden="true" {...stylex.attrs(chapterStyles.lifecycleGrain)} />
                    {homeExamples.map((example, index) => (
                        <Chapter example={example} index={index} />
                    ))}
                </div>

                <div aria-hidden="true" {...stylex.attrs(chapterStyles.worldGrain)} />
            </article>
        </Shell>
    );
}

/// Render the publication cover and installation procedure.
function Cover() {
    return (
        <section {...stylex.attrs(coverStyles.cover)}>
            <div {...stylex.attrs(coverStyles.frame)}>
                {/* Series masthead */}
                <h1 aria-label="Destack" {...stylex.attrs(coverStyles.title)}>
                    <span aria-hidden="true" {...stylex.attrs(coverStyles.titleFirst)}>D</span>
                    <span aria-hidden="true">E</span>
                    <span aria-hidden="true">S</span>
                    <span aria-hidden="true">T</span>
                    <span aria-hidden="true">A</span>
                    <span aria-hidden="true">C</span>
                    <span aria-hidden="true" {...stylex.attrs(coverStyles.titleLast)}>K</span>
                </h1>

                {/* Publication navigation */}
                <nav aria-label="Destack navigation" {...stylex.attrs(coverStyles.navigation)}>
                    <div {...stylex.attrs(coverStyles.links, coverStyles.primaryLinks)}>
                        <A {...stylex.attrs(coverStyles.link)} href="/docs/">docs</A>
                        <A {...stylex.attrs(coverStyles.link)} href="/blog/">blog</A>
                        <button
                            {...stylex.attrs(coverStyles.link)}
                            onClick={() =>
                                document.dispatchEvent(new CustomEvent(commandEvents.open))
                            }
                            type="button"
                        >
                            search
                        </button>
                    </div>

                    <div {...stylex.attrs(coverStyles.links, coverStyles.socialLinks)}>
                        {socialLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                style={coverStyles.link}
                                shortcut={shortcut}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label}
                            </SiteLink>
                        ))}
                    </div>
                </nav>

                {/* Main proposition */}
                <div {...stylex.attrs(coverStyles.hero)}>
                    <header {...stylex.attrs(coverStyles.heroCopy)}>
                        <p {...stylex.attrs(pitchStyles.pitch)}>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineThe)}>the</span>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineAbsurdly)}>absurdly</span>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineIntegrated)}>integrated</span>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineOpen)}>open</span>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineComputing)}>computing</span>
                            <span {...stylex.attrs(pitchStyles.line, pitchStyles.lineStack)}>stack</span>
                        </p>
                    </header>

                    <Installation />
                </div>
            </div>
        </section>
    );
}

/// Render the first numbered installation procedure.
function Installation() {
    return (
        <aside {...stylex.attrs(installationStyles.installation)}>
            <header {...stylex.attrs(installationStyles.header)}>
                <span {...stylex.attrs(installationStyles.number)}>00</span>
                <strong {...stylex.attrs(installationStyles.title)}>Install Destack</strong>
            </header>

            <div {...stylex.attrs(installationStyles.command)}>
                <span aria-hidden="true">$</span>
                <code {...stylex.attrs(installationStyles.commandCode)}>{installCommand}</code>
            </div>

            <footer {...stylex.attrs(installationStyles.footer)}>
                <a {...stylex.attrs(installationStyles.action)} href="/docs/">
                    get started <span aria-hidden="true" {...stylex.attrs(installationStyles.arrow)}>→</span>
                </a>
            </footer>
        </aside>
    );
}

/// Render the restrained planetary field behind the complete page.
function Universe() {
    return (
        <div aria-hidden="true" {...stylex.attrs(universeStyles.universe)}>
            <svg
                {...stylex.attrs(universeStyles.view)}
                preserveAspectRatio="xMidYMid slice"
                viewBox="0 0 1600 1100"
                xmlns="http://www.w3.org/2000/svg"
            >
                <defs>
                    <clipPath id="home-planet-clip">
                        <circle cx="1180" cy="760" r="520" />
                    </clipPath>

                    <pattern
                        id="home-object-grain"
                        height="180"
                        patternUnits="userSpaceOnUse"
                        width="180"
                    >
                        <image height="180" href="/grain.svg" width="180" />
                    </pattern>
                </defs>

                {/* Distant star field */}
                <g {...stylex.attrs(universeStyles.object, universeStyles.stars)} fill={tokens.cream}>
                    <g {...stylex.attrs(universeStyles.drift, universeStyles.driftStars)}>
                        <Star cx="116" cy="154" radius="2.5" twinkles />
                        <Star cx="345" cy="86" radius="1.5" />
                        <Star cx="570" cy="192" radius="2" twinkles />
                        <Star cx="820" cy="112" radius="1.5" />
                        <Star cx="1070" cy="72" radius="2.5" twinkles />
                        <Star cx="1260" cy="210" radius="1.25" />
                        <Star cx="1425" cy="155" radius="1.5" twinkles />
                        <Star cx="1535" cy="345" radius="2" />
                        <Star cx="1480" cy="475" radius="2.5" twinkles />
                        <Star cx="1330" cy="565" radius="1.5" />
                        <Star cx="240" cy="590" radius="2" twinkles />
                        <Star cx="80" cy="430" radius="1.5" />
                        <Star cx="425" cy="385" radius="1.25" twinkles />
                        <Star cx="675" cy="520" radius="1.75" />
                        <Star cx="925" cy="430" radius="1.5" twinkles />
                        <Star cx="96" cy="910" radius="1.5" />
                        <Star cx="290" cy="800" radius="2" twinkles />
                        <Star cx="470" cy="1015" radius="2" />
                        <Star cx="720" cy="920" radius="1.25" twinkles />
                        <Star cx="970" cy="1040" radius="1.5" />
                        <Star cx="1170" cy="880" radius="2" twinkles />
                        <Star cx="1370" cy="970" radius="1.5" />
                        <Star cx="1510" cy="830" radius="1.25" twinkles />
                    </g>
                </g>

                {/* Independently moving ring and planet */}
                <g transform="translate(180 110)">
                    <g {...stylex.attrs(universeStyles.object, universeStyles.ring)}>
                        <g {...stylex.attrs(universeStyles.drift, universeStyles.driftRing)}>
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke={tokens.ink}
                                stroke-width="92"
                                transform="rotate(-12 1180 760)"
                            />
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke={tokens.creamDeep}
                                stroke-width="86"
                                transform="rotate(-12 1180 760)"
                            />
                        </g>
                    </g>

                    <g {...stylex.attrs(universeStyles.object, universeStyles.planet)}>
                        <g {...stylex.attrs(universeStyles.drift, universeStyles.driftPlanet)}>
                            <circle cx="1180" cy="760" fill={tokens.orange} r="520" />
                            <g clip-path="url(#home-planet-clip)">
                                <path
                                    d="M610 695C990 830 1430 810 1770 660"
                                    fill="none"
                                    stroke={tokens.rust}
                                    stroke-width="42"
                                />
                                <rect
                                    {...stylex.attrs(universeStyles.texture)}
                                    fill="url(#home-object-grain)"
                                    height="1040"
                                    width="1040"
                                    x="660"
                                    y="240"
                                />
                            </g>
                            <circle
                                cx="1180"
                                cy="760"
                                fill="none"
                                r="520"
                                stroke={tokens.ink}
                                stroke-width="3"
                                vector-effect="non-scaling-stroke"
                            />
                        </g>
                    </g>
                </g>
            </svg>
            <div {...stylex.attrs(universeStyles.overlay)} />
        </div>
    );
}

type StarProps = {
    /// The horizontal SVG coordinate.
    cx: string;

    /// The vertical SVG coordinate.
    cy: string;

    /// The star radius.
    radius: string;

    /// Whether the star changes brightness.
    twinkles?: boolean;
};

/// Render one distant star.
function Star(props: StarProps) {
    return (
        <circle
            {...stylex.attrs(universeStyles.star, props.twinkles && universeStyles.twinkle)}
            cx={props.cx}
            cy={props.cy}
            r={props.radius}
        />
    );
}

/// Render one numbered system chapter.
function Chapter(props: ChapterProps) {
    const number = String(props.index + 1).padStart(2, "0");

    return (
        <section {...stylex.attrs(chapterStyles.chapter)} id={props.example.action}>
            <div {...stylex.attrs(chapterStyles.frame)}>
                {/* Chapter brief */}
                <header {...stylex.attrs(chapterStyles.brief)}>
                    <div {...stylex.attrs(chapterStyles.heading)}>
                        <span {...stylex.attrs(chapterStyles.headingNumber)}>{number}</span>
                        <h2 {...stylex.attrs(chapterStyles.headingTitle)}>{props.example.action}</h2>
                    </div>

                    <div {...stylex.attrs(chapterStyles.copy)}>
                        <strong {...stylex.attrs(chapterStyles.claim)}>{props.example.claim}</strong>
                        <p {...stylex.attrs(chapterStyles.description)}>{props.example.description}</p>
                    </div>

                    <div {...stylex.attrs(chapterStyles.replacements)}>
                        <span {...stylex.attrs(chapterStyles.replacementLabel)}>replaces</span>
                        <ul aria-label="Systems consolidated by this chapter" {...stylex.attrs(chapterStyles.replacementList)}>
                            {props.example.replacements.map((replacement, index) => (
                                <li {...stylex.attrs(chapterStyles.replacement)}>
                                    {index > 0 && <span aria-hidden="true" {...stylex.attrs(chapterStyles.replacementSeparator)}>/</span>}
                                    {replacement}
                                </li>
                            ))}
                        </ul>
                    </div>
                </header>

                {/* Executable specimen */}
                <CodePlate example={props.example} />
            </div>
        </section>
    );
}

/// Render one executable code specimen and its observed result.
function CodePlate(props: { example: HomeExample }) {
    const lines = homeHighlights[props.example.action];

    return (
        <figure {...stylex.attrs(plateStyles.plate)}>
            <figcaption {...stylex.attrs(plateStyles.caption)}>
                <strong {...stylex.attrs(plateStyles.title)}>{props.example.source.title}</strong>
                <span {...stylex.attrs(plateStyles.format)}>
                    {homeLanguageLabels[props.example.source.language]}
                </span>
            </figcaption>

            <div {...stylex.attrs(plateStyles.source)} data-syntax>
                <ol aria-label={props.example.source.title} {...stylex.attrs(plateStyles.lines)}>
                    {lines.map((line, index) => (
                        <li {...stylex.attrs(plateStyles.line)}>
                            <span aria-hidden="true" {...stylex.attrs(plateStyles.lineNumber)}>{index + 1}</span>
                            <code {...stylex.attrs(plateStyles.code)} innerHTML={line || " "} />
                        </li>
                    ))}
                </ol>
            </div>

            <footer {...stylex.attrs(plateStyles.result)}>
                <span>{props.example.result.title}</span>
                <pre {...stylex.attrs(plateStyles.resultText)}>{props.example.result.text}</pre>
            </footer>
        </figure>
    );
}
