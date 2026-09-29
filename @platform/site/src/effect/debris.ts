import type { Orbit } from "../home/plate";
import { pageScroll } from "./gl";

/** The milliseconds a shard takes to rise from its berg into the ring, at the least. */
const riseTime = 1700;
/** The most milliseconds a shard waits before it rises, so the shards leave one after another. */
const riseSpread = 700;
/** The milliseconds an arrived shard takes to settle into the ice band. */
const settleTime = 700;
/** The milliseconds a shard takes to fall from the ring back to its berg, at the least. */
const fallTime = 1000;
/** The most milliseconds a shard waits before it falls. */
const fallSpread = 300;
/** The milliseconds a landed shard takes to melt back into the reforming ice. */
const meltTime = 450;
/** How far above the higher of its two ends a shard's path arcs, in CSS pixels. */
const arcHeight = 140;
/** The ice band's place as a share of the ring's reach, just outside the orange ring. */
const bandLane = 1.16;
/** The ice band's width as a share of the planet's radius. */
const bandWidth = 0.075;
/** The gap cut around the ice band where it crosses in front of the globe, as a share of the planet's radius. */
const bandGap = 0.05;
/** The ice band's color. */
const bandTone = "#dcedf2";
/** The deep space color the gap is cut in. */
const space = "#081723";
/** The ice tones of the shards. */
const tones = ["#ffffff", "#e6f3f7", "#cfe7ee"];
/** The soft ink edge of the shards, so they read over the cream page too. */
const edge = "rgb(18 49 60 / 45%)";
/** How far behind a flying shard its trail reaches, in frames of motion. */
const trailLength = 4;

/** A point in page pixels. */
type Point = { x: number; y: number };

/** Where a shard is in its journey between its berg and the ring. */
type Leg = "rising" | "orbiting" | "falling" | "melting";

/** One shard of ice: where it broke off, its place in the ring, and its shape. */
type Shard = {
    /** Where the shard broke off its berg, in page pixels. */
    home: Point;
    /** Where the shard's current leg began, in page pixels. */
    from: Point;
    /** Where the shard was on the previous frame, in page pixels, for its trail. */
    last: Point | undefined;
    /** The leg the shard is on. */
    leg: Leg;
    /** When the current leg starts, in milliseconds. */
    at: number;
    /** How long the current leg takes, in milliseconds. */
    duration: number;
    /** The shard's angle around the ring when the orbit clock starts, in radians. */
    phase: number;
    /** How fast the shard circles the ring, in radians per second. */
    speed: number;
    /** How fast the shard tumbles, in radians per second. */
    spin: number;
    /** The shard's radius in CSS pixels. */
    size: number;
    /** The shard's fill. */
    tone: string;
};

/** Carry shattered ice up into a band around the planet's ring, and bring it home again when the ice reforms. */
export class Debris {
    /** The canvas the shards are drawn on, fixed over the page. */
    canvas: HTMLCanvasElement;
    /** The drawing context. */
    context: CanvasRenderingContext2D;
    /** The shards in flight, in the band, or melting home. */
    shards: Shard[];
    /** Return the planet and its ring on screen, or nothing when the page shows none. */
    orbit: () => Orbit | undefined;
    /** The pending animation frame, if any. */
    frame: number | undefined;
    /** Draw once more, when the page scrolls or resizes under the resting band. */
    redraw: () => void;

    /** Create debris on a canvas, circling the planet the orbit function finds. */
    constructor(canvas: HTMLCanvasElement, orbit: () => Orbit | undefined) {
        // hold the canvas and start with no shards
        const context = canvas.getContext("2d");
        if (context === null) {
            throw new Error("canvas drawing is unavailable");
        }
        this.canvas = canvas;
        this.context = context;
        this.shards = [];
        this.orbit = orbit;
        this.frame = undefined;
        this.redraw = () => this.request();
        window.addEventListener("scroll", this.redraw, { passive: true });
        window.addEventListener("resize", this.redraw);
    }

    /** Break shards off the given points on the ice and send them up into the band. */
    rise(origins: readonly Point[]) {
        // give each shard its start, its place in the band, and its size
        const now = performance.now();
        this.shards = origins.map((home) => ({
            home,
            from: home,
            last: undefined,
            leg: "rising",
            at: now + Math.random() * riseSpread,
            duration: riseTime * (0.85 + Math.random() * 0.4),
            phase: Math.random() * Math.PI * 2,
            speed: 0.16 + Math.random() * 0.06,
            spin: (Math.random() - 0.5) * 3,
            size: 1.2 + Math.random() * 1.6,
            tone: tones[Math.floor(Math.random() * tones.length)],
        }));
        this.request();
    }

    /** Lift every shard back out of the band and bring it down to where it broke off. */
    fall() {
        // start each shard's fall from wherever it is now
        const now = performance.now();
        const orbit = this.orbit();
        for (const shard of this.shards) {
            shard.from = this.placeOf(shard, now, orbit);
            shard.last = undefined;
            shard.leg = "falling";
            shard.at = now + Math.random() * fallSpread;
            shard.duration = fallTime * (0.85 + Math.random() * 0.4);
        }
        this.request();
    }

    /** Schedule the next frame once. */
    request() {
        if (this.frame === undefined) {
            this.frame = requestAnimationFrame((now) => this.draw(now));
        }
    }

    /** Stop drawing and stop following the page. */
    stop() {
        window.removeEventListener("scroll", this.redraw);
        window.removeEventListener("resize", this.redraw);
        if (this.frame !== undefined) {
            cancelAnimationFrame(this.frame);
            this.frame = undefined;
        }
    }

    /** Return where a shard is at a time, in page pixels, moving each leg along to the next as it ends. */
    placeOf(shard: Shard, now: number, orbit: Orbit | undefined): Point {
        // follow the band while orbiting, or sit where the leg began without a planet
        const progress = Math.max(0, Math.min(1, (now - shard.at) / shard.duration));
        if (shard.leg === "orbiting" || !orbit) {
            return orbit ? bandPoint(shard, now, orbit) : shard.from;
        }
        // arc up from the berg to the shard's moving place in the band, then settle into it
        else if (shard.leg === "rising") {
            if (progress >= 1) {
                Object.assign(shard, { leg: "orbiting", at: now, duration: settleTime });
            }

            return arc(shard.from, bandPoint(shard, now, orbit), ease(progress));
        }
        // arc down from the band to the berg, then melt there
        else if (shard.leg === "falling") {
            if (progress >= 1) {
                Object.assign(shard, {
                    leg: "melting",
                    at: now,
                    duration: meltTime,
                    from: shard.home,
                });
            }

            return arc(shard.from, shard.home, ease(progress));
        }
        // rest on the berg while melting
        else {
            return shard.home;
        }
    }

    /** Draw one frame, and keep going while any shard remains. */
    draw(now: number) {
        // size the canvas to the window at the screen's density and clear it
        this.frame = undefined;
        const canvas = this.canvas;
        const density = Math.min(window.devicePixelRatio, 1.5);
        const width = Math.round(canvas.clientWidth * density);
        const height = Math.round(canvas.clientHeight * density);
        if (canvas.width !== width || canvas.height !== height) {
            canvas.width = width;
            canvas.height = height;
        }
        const context = this.context;
        context.setTransform(density, 0, 0, density, 0, 0);
        context.clearRect(0, 0, canvas.clientWidth, canvas.clientHeight);

        // drop shards that have melted away
        this.shards = this.shards.filter(
            (shard) => shard.leg !== "melting" || now - shard.at < shard.duration,
        );

        // draw the band as solid as the share of shards settled into it
        const orbit = this.orbit();
        const settled = this.shards.map((shard) =>
            shard.leg === "orbiting" ? Math.min(1, (now - shard.at) / shard.duration) : 0,
        );
        const solidity =
            settled.reduce((sum, share) => sum + share, 0) / Math.max(1, this.shards.length);
        if (orbit && solidity > 0) {
            drawBand(context, orbit, solidity, canvas.clientWidth, canvas.clientHeight);
        }

        // draw each shard with a short trail, fading into the band as it settles
        this.shards.forEach((shard, index) => {
            // place the shard, remember it for the next trail, and skip it when unseen or behind the globe
            const alpha = shardAlpha(shard, now, settled[index]);
            const page = this.placeOf(shard, now, orbit);
            const last = shard.last;
            shard.last = page;
            if (alpha <= 0 || (orbit && shard.leg === "orbiting" && isBehind(shard, now, orbit))) {
                return;
            }
            drawShard(context, shard, page, last, alpha, now / 1000);
        });

        // keep drawing while any shard still moves, and rest once the band is solid until the page scrolls
        if (this.shards.length > 0 && solidity < 1) {
            this.request();
        }
    }
}

/** Return how visible a shard is: fading in as it leaves the ice, out as it settles into the band or melts home. */
function shardAlpha(shard: Shard, now: number, settled: number) {
    // fade in over the first moments of a rise or fall
    const since = now - shard.at;
    if (shard.leg === "rising" || shard.leg === "falling") {
        return Math.min(1, Math.max(0, since / 200));
    }
    // melt away on the berg
    else if (shard.leg === "melting") {
        return 1 - since / shard.duration;
    }
    // settle into the band
    else {
        return 1 - settled;
    }
}

/** Draw one shard of ice as a small tumbling chip trailing a faint streak, in client pixels. */
function drawShard(
    context: CanvasRenderingContext2D,
    shard: Shard,
    page: Point,
    last: Point | undefined,
    alpha: number,
    seconds: number,
) {
    // place the shard on screen
    const x = page.x - pageScroll.x;
    const y = page.y - pageScroll.y;

    // streak a trail back along the shard's motion
    if (last) {
        const trailX = x - (page.x - last.x) * trailLength;
        const trailY = y - (page.y - last.y) * trailLength;
        const trail = context.createLinearGradient(trailX, trailY, x, y);
        trail.addColorStop(0, "rgb(230 243 247 / 0%)");
        trail.addColorStop(1, `rgb(230 243 247 / ${(alpha * 0.55).toFixed(3)})`);
        context.strokeStyle = trail;
        context.lineWidth = shard.size;
        context.lineCap = "round";
        context.beginPath();
        context.moveTo(trailX, trailY);
        context.lineTo(x, y);
        context.stroke();
    }

    // draw the chip as a tumbling diamond
    const turn = shard.spin * seconds;
    const size = shard.size * 1.3;
    context.globalAlpha = alpha;
    context.beginPath();
    for (let corner = 0; corner < 4; corner++) {
        const angle = turn + (corner * Math.PI) / 2;
        const reach = corner % 2 === 0 ? size : size * 0.6;
        const cornerX = x + Math.cos(angle) * reach;
        const cornerY = y + Math.sin(angle) * reach;
        if (corner === 0) {
            context.moveTo(cornerX, cornerY);
        } else {
            context.lineTo(cornerX, cornerY);
        }
    }
    context.closePath();
    context.fillStyle = shard.tone;
    context.fill();
    context.lineWidth = 0.75;
    context.strokeStyle = edge;
    context.stroke();
    context.globalAlpha = 1;
}

/** Draw the ice band around the ring as flat as the mark: hidden behind the globe, cut free where it crosses in front. */
function drawBand(
    context: CanvasRenderingContext2D,
    orbit: Orbit,
    solidity: number,
    width: number,
    height: number,
) {
    // size the band just outside the orange ring
    const across = orbit.across * bandLane;
    const down = orbit.down * bandLane;
    const thickness = orbit.radius * bandWidth;
    context.globalAlpha = solidity;
    context.lineCap = "butt";

    // draw the far half everywhere but over the globe
    context.save();
    context.beginPath();
    context.rect(0, 0, width, height);
    context.arc(orbit.x, orbit.y, orbit.radius, 0, Math.PI * 2);
    context.clip("evenodd");
    context.beginPath();
    context.ellipse(orbit.x, orbit.y, across, down, orbit.tilt, Math.PI, Math.PI * 2);
    context.strokeStyle = bandTone;
    context.lineWidth = thickness;
    context.stroke();
    context.restore();

    // cut a gap in the globe where the near half crosses it, then draw the near half over it
    context.save();
    context.beginPath();
    context.arc(orbit.x, orbit.y, orbit.radius, 0, Math.PI * 2);
    context.clip();
    context.beginPath();
    context.ellipse(orbit.x, orbit.y, across, down, orbit.tilt, 0, Math.PI);
    context.strokeStyle = space;
    context.lineWidth = thickness + orbit.radius * bandGap * 2;
    context.stroke();
    context.restore();
    context.beginPath();
    context.ellipse(orbit.x, orbit.y, across, down, orbit.tilt, 0, Math.PI);
    context.strokeStyle = bandTone;
    context.lineWidth = thickness;
    context.stroke();
    context.globalAlpha = 1;
}

/** Return a shard's place in the band at a time, in page pixels. */
function bandPoint(shard: Shard, now: number, orbit: Orbit): Point {
    // circle the band's ellipse, then tilt it with the ring
    const angle = shard.phase + (shard.speed * now) / 1000;
    const across = Math.cos(angle) * orbit.across * bandLane;
    const down = Math.sin(angle) * orbit.down * bandLane;

    return {
        x: orbit.x + across * Math.cos(orbit.tilt) - down * Math.sin(orbit.tilt) + pageScroll.x,
        y: orbit.y + across * Math.sin(orbit.tilt) + down * Math.cos(orbit.tilt) + pageScroll.y,
    };
}

/** Return whether an orbiting shard is on the band's far side and over the globe. */
function isBehind(shard: Shard, now: number, orbit: Orbit) {
    // find the shard on the band, and test the far side against the globe
    const angle = shard.phase + (shard.speed * now) / 1000;
    const place = bandPoint(shard, now, orbit);
    const x = place.x - pageScroll.x;
    const y = place.y - pageScroll.y;

    return Math.sin(angle) < 0 && Math.hypot(x - orbit.x, y - orbit.y) < orbit.radius;
}

/** Return a point along an arc that rises above both of its ends, in page pixels. */
function arc(from: Point, to: Point, progress: number): Point {
    // bend the path through a control point above the higher end
    const control = { x: (from.x + to.x) / 2, y: Math.min(from.y, to.y) - arcHeight };
    const rest = 1 - progress;

    return {
        x: rest * rest * from.x + 2 * rest * progress * control.x + progress * progress * to.x,
        y: rest * rest * from.y + 2 * rest * progress * control.y + progress * progress * to.y,
    };
}

/** Ease in and out, slow at both ends. */
function ease(progress: number) {
    return progress < 0.5 ? 4 * progress ** 3 : 1 - (-2 * progress + 2) ** 3 / 2;
}
