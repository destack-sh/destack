import { Download } from "./download/download";
import { Horizon, Scene } from "./scene";
import "./home.css";

/** Render the illustrated platform introduction. */
export function Cover() {
    return (
        <div class="universe">
            {/* compose the title and platform invitation in one sky */}
            <section class="landing" aria-label="Introducing Destack">
                <p class="platform-introduction">
                    <strong>Destack is a personal software platform that lets you <u>own</u> your software.</strong>
                    <br />
                    Unify your apps, bring your agents, extend and customise just what you need.
                    <br />
                    Run everything on your computer, your server, our server, or anything in between.
                </p>
                <Scene />
                <div class="invitation">
                    <Horizon />
                    <header class="wordmark">
                        <h1 aria-label="Destack">
                            {"DESTACK".split("").map((letter) => (
                                <span aria-hidden="true">{letter}</span>
                            ))}
                        </h1>
                    </header>
                    <div class="landing-actions">
                        <Download />
                    </div>
                </div>
            </section>
        </div>
    );
}
