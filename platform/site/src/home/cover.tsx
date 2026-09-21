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
                    <p>
                        <span>Absurdly integrated</span>
                        <span>
                            <em>Fully</em> hackable
                        </span>
                    </p>
                    <h1 aria-label="Destack">
                        {"DESTACK".split("").map((letter) => (
                            <span aria-hidden="true">{letter}</span>
                        ))}
                    </h1>
                    <p>
                        <span>
                            <u>Open</u> source
                        </span>
                        <span>Computing stack</span>
                    </p>
                </header>
                <div class="invitation">
                    <Horizon />
                    <h2>Personal software platform</h2>
                    <div class="landing-actions">
                        <Download />
                    </div>
                </div>
            </section>
        </div>
    );
}
