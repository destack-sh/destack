import { A } from "@solidjs/router";
import { createMemo } from "solid-js";

import { commandEvents } from "../command/command";
import { type HomeExample, homeExamples, installCommand } from "../content/site";
import { SiteLink } from "../navigation/link";
import { socialLinks } from "../navigation/navigation";
import { highlightExample } from "../site/highlight";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import "../style/home.css";

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
        <Shell class="home-shell">
            <Seo
                description={
                    "Destack is a universal software engine for building complete " +
                    "software systems."
                }
            />

            <article class="home-world">
                <Universe />
                <Cover />

                {/* System chapters */}
                <div aria-label="Destack system lifecycle" class="home-lifecycle">
                    {homeExamples.map((example, index) => (
                        <Chapter example={example} index={index} />
                    ))}
                </div>
            </article>
        </Shell>
    );
}

/// Render the publication cover and installation procedure.
function Cover() {
    return (
        <section class="home-cover">
            <div class="home-cover__frame">
                {/* Series masthead */}
                <h1 aria-label="Destack" class="display home-title">
                    <span aria-hidden="true">D</span>
                    <span aria-hidden="true">E</span>
                    <span aria-hidden="true">S</span>
                    <span aria-hidden="true">T</span>
                    <span aria-hidden="true">A</span>
                    <span aria-hidden="true">C</span>
                    <span aria-hidden="true">K</span>
                </h1>

                {/* Publication navigation */}
                <nav aria-label="Destack navigation" class="home-cover__navigation">
                    <div class="home-cover__links home-cover__links--primary">
                        <A href="/docs/">docs</A>
                        <A href="/blog/">blog</A>
                        <button
                            onClick={() =>
                                document.dispatchEvent(new CustomEvent(commandEvents.open))
                            }
                            type="button"
                        >
                            search
                        </button>
                    </div>

                    <div class="home-cover__links home-cover__links--social">
                        {socialLinks.map(({ label, href, shortcut }) => (
                            <SiteLink
                                href={href}
                                shortcut={shortcut}
                                title={`Alt+${shortcut.toUpperCase()}: ${label}`}
                            >
                                {label}
                            </SiteLink>
                        ))}
                    </div>
                </nav>

                {/* Main proposition */}
                <div class="home-hero">
                    <header class="home-hero__copy">
                        <p class="display home-pitch">
                            <span>the</span>
                            <span>absurdly</span>
                            <span>integrated</span>
                            <span>open</span>
                            <span>computing</span>
                            <span>stack</span>
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
        <aside class="home-installation">
            <header class="home-installation__header">
                <span class="home-installation__number">00</span>
                <strong>Install Destack</strong>
            </header>

            <div class="home-installation__command">
                <span aria-hidden="true">$</span>
                <code>{installCommand}</code>
            </div>

            <footer class="home-installation__footer">
                <a href="/docs/">
                    get started <span aria-hidden="true">→</span>
                </a>
            </footer>
        </aside>
    );
}

/// Render the restrained planetary field behind the complete page.
function Universe() {
    return (
        <div aria-hidden="true" class="home-universe">
            <svg
                class="home-universe__view"
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
                <g class="home-object home-object--stars" fill="var(--color-cream)">
                    <g class="home-object__drift">
                        <circle class="home-star home-star--twinkle" cx="116" cy="154" r="2.5" />
                        <circle class="home-star" cx="345" cy="86" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="570" cy="192" r="2" />
                        <circle class="home-star" cx="820" cy="112" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="1070" cy="72" r="2.5" />
                        <circle class="home-star" cx="1260" cy="210" r="1.25" />
                        <circle class="home-star home-star--twinkle" cx="1425" cy="155" r="1.5" />
                        <circle class="home-star" cx="1535" cy="345" r="2" />
                        <circle class="home-star home-star--twinkle" cx="1480" cy="475" r="2.5" />
                        <circle class="home-star" cx="1330" cy="565" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="240" cy="590" r="2" />
                        <circle class="home-star" cx="80" cy="430" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="425" cy="385" r="1.25" />
                        <circle class="home-star" cx="675" cy="520" r="1.75" />
                        <circle class="home-star home-star--twinkle" cx="925" cy="430" r="1.5" />
                        <circle class="home-star" cx="96" cy="910" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="290" cy="800" r="2" />
                        <circle class="home-star" cx="470" cy="1015" r="2" />
                        <circle class="home-star home-star--twinkle" cx="720" cy="920" r="1.25" />
                        <circle class="home-star" cx="970" cy="1040" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="1170" cy="880" r="2" />
                        <circle class="home-star" cx="1370" cy="970" r="1.5" />
                        <circle class="home-star home-star--twinkle" cx="1510" cy="830" r="1.25" />
                    </g>
                </g>

                {/* Independently moving ring and planet */}
                <g transform="translate(180 110)">
                    <g class="home-object home-object--ring">
                        <g class="home-object__drift">
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke="var(--color-ink)"
                                stroke-width="92"
                                transform="rotate(-12 1180 760)"
                            />
                            <ellipse
                                cx="1180"
                                cy="760"
                                fill="none"
                                rx="1470"
                                ry="245"
                                stroke="var(--color-cream-deep)"
                                stroke-width="86"
                                transform="rotate(-12 1180 760)"
                            />
                        </g>
                    </g>

                    <g class="home-object home-object--planet">
                        <g class="home-object__drift">
                            <circle cx="1180" cy="760" fill="var(--color-orange)" r="520" />
                            <g clip-path="url(#home-planet-clip)">
                                <path
                                    d="M610 695C990 830 1430 810 1770 660"
                                    fill="none"
                                    stroke="var(--color-rust)"
                                    stroke-width="42"
                                />
                                <rect
                                    class="home-object__texture"
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
                                stroke="var(--color-ink)"
                                stroke-width="3"
                                vector-effect="non-scaling-stroke"
                            />
                        </g>
                    </g>
                </g>
            </svg>
        </div>
    );
}

/// Render one numbered system chapter.
function Chapter(props: ChapterProps) {
    const number = String(props.index + 1).padStart(2, "0");

    return (
        <section class="home-chapter" id={props.example.action}>
            <div class="home-chapter__frame">
                {/* Chapter brief */}
                <header class="home-chapter__brief">
                    <div class="home-chapter__heading">
                        <span>{number}</span>
                        <h2>{props.example.action}</h2>
                    </div>

                    <div class="home-chapter__copy">
                        <strong>{props.example.claim}</strong>
                        <p>{props.example.description}</p>
                    </div>

                    <div class="home-chapter__replacements">
                        <span>replaces</span>
                        <ul aria-label="Systems consolidated by this chapter">
                            {props.example.replacements.map((replacement) => (
                                <li>{replacement}</li>
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
    const lines = createMemo(() =>
        highlightExample(props.example.source.code, props.example.source.language),
    );

    return (
        <figure class="home-code-plate">
            <figcaption>
                <strong>{props.example.source.title}</strong>
                <span>{props.example.source.language}</span>
            </figcaption>

            <div class="home-code-plate__source syntax">
                <ol aria-label={props.example.source.title}>
                    {lines().map((line, index) => (
                        <li>
                            <span aria-hidden="true">{index + 1}</span>
                            <code innerHTML={line || " "} />
                        </li>
                    ))}
                </ol>
            </div>

            <footer class="home-code-plate__result">
                <span>{props.example.result.title}</span>
                <pre>{props.example.result.text}</pre>
            </footer>
        </figure>
    );
}
