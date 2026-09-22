import { Download } from "./download/download";
import { Horizon, Scene } from "./scene";
import "./home.css";

/** Render the illustrated platform introduction. */
export function Cover() {
    return (
        <div class="universe">
            <Scene />
            {/* compose the title and platform invitation in one sky */}
            <section class="landing" aria-label="Introducing Destack">
                <header class="wordmark">
                    <h1 aria-label="Destack">
                        {"DESTACK".split("").map((letter) => (
                            <span aria-hidden="true">{letter}</span>
                        ))}
                    </h1>
                </header>
                <div class="invitation">
                    <Horizon />
                    <h2 aria-label="Personal software platform">
                        <svg
                            viewBox="0 0 1000 155"
                            role="img"
                            aria-label="Personal software platform. Absurdly integrated. Fully hackable. Open source computing stack."
                        >
                            <defs>
                                <path id="platform-curve" d="M0 115Q500 65 1000 115" />
                                <path id="platform-above" d="M0 60Q500 10 1000 60" />
                                <path id="platform-below" d="M0 149Q500 99 1000 149" />
                            </defs>
                            <text class="platform-caption" fill="#ec8854">
                                <textPath href="#platform-above" startOffset="1%">
                                    Absurdly integrated
                                </textPath>
                            </text>
                            <text class="platform-caption">
                                <textPath
                                    href="#platform-above"
                                    startOffset="99%"
                                    text-anchor="end"
                                >
                                    <tspan font-style="italic">Fully</tspan> hackable
                                </textPath>
                            </text>
                            <text textLength="980" lengthAdjust="spacing">
                                <textPath
                                    href="#platform-curve"
                                    startOffset="50%"
                                    text-anchor="middle"
                                >
                                    Personal software platform
                                </textPath>
                            </text>
                            <text class="platform-caption platform-caption-below">
                                <textPath href="#platform-below" startOffset="1%">
                                    <tspan text-decoration="underline">Open</tspan> source
                                </textPath>
                            </text>
                            <text class="platform-caption platform-caption-below" fill="#ec8854">
                                <textPath
                                    href="#platform-below"
                                    startOffset="99%"
                                    text-anchor="end"
                                >
                                    Computing stack
                                </textPath>
                            </text>
                        </svg>
                    </h2>
                    <div class="landing-actions">
                        <Download />
                    </div>
                </div>
            </section>
        </div>
    );
}
