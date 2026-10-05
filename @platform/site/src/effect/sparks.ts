/** The most particles alive at once. */
const capacity = 400;
/** The pull of gravity on a spark, in CSS pixels per second squared. */
const gravity = 260;

/** One particle of light. */
type Particle = {
    /** The position across, in CSS pixels. */
    x: number;
    /** The position down, in CSS pixels. */
    y: number;
    /** The speed across, in CSS pixels per second. */
    speedX: number;
    /** The speed down, in CSS pixels per second. */
    speedY: number;
    /** The remaining life, in seconds. */
    life: number;
    /** The full life, in seconds. */
    span: number;
};

/** Draw sparks on a canvas that burst and fall. */
export class Sparks {
    /** The canvas the particles are drawn on. */
    canvas: HTMLCanvasElement;
    /** The drawing context. */
    context: CanvasRenderingContext2D;
    /** The particles alive, at most the capacity. */
    particles: Particle[];
    /** The pending animation frame, if any. */
    frame: number | undefined;
    /** The time of the last frame in milliseconds. */
    last: number;

    /** Create the particles on a canvas. */
    constructor(canvas: HTMLCanvasElement) {
        // take the 2D context and start without particles
        const context = canvas.getContext("2d");
        if (!context) {
            throw new TypeError("the sparks canvas has no 2D context");
        }
        this.canvas = canvas;
        this.context = context;
        this.particles = [];
        this.frame = undefined;
        this.last = 0;
    }

    /** Burst a spray of sparks from a point, flung up and out. */
    burst(x: number, y: number, count: number) {
        for (let index = 0; index < count; index++) {
            const angle = -Math.PI / 2 + (Math.random() - 0.5) * 2.4;
            const speed = 80 + Math.random() * 220;
            this.add(
                x + (Math.random() - 0.5) * 40,
                y,
                Math.cos(angle) * speed,
                Math.sin(angle) * speed,
                0.6 + Math.random() * 0.7,
            );
        }
        this.request();
    }

    /** Add one particle, replacing the oldest when full. */
    add(x: number, y: number, speedX: number, speedY: number, life: number) {
        // take a free place, or replace a random particle when full
        const particle = { x, y, speedX, speedY, life, span: life };
        if (this.particles.length < capacity) {
            this.particles.push(particle);
        } else {
            this.particles[Math.floor(Math.random() * capacity)] = particle;
        }
    }

    /** Schedule the next frame once. */
    request() {
        if (this.frame === undefined) {
            this.last = performance.now();
            this.frame = requestAnimationFrame((now) => this.draw(now));
        }
    }

    /** Stop drawing. */
    stop() {
        if (this.frame !== undefined) {
            cancelAnimationFrame(this.frame);
            this.frame = undefined;
        }
    }

    /** Move and draw every particle, and keep going while any are alive. */
    draw(now: number) {
        // clear the pending frame and measure the time since the last
        this.frame = undefined;
        const elapsed = Math.min(0.05, (now - this.last) / 1000);
        this.last = now;

        // match the backing store to the displayed size
        const scale = Math.min(window.devicePixelRatio, 2);
        const width = this.canvas.clientWidth;
        const height = this.canvas.clientHeight;
        if (this.canvas.width !== Math.round(width * scale)) {
            this.canvas.width = Math.round(width * scale);
            this.canvas.height = Math.round(height * scale);
        }
        const context = this.context;
        context.setTransform(scale, 0, 0, scale, 0, 0);
        context.clearRect(0, 0, width, height);
        context.globalCompositeOperation = "lighter";

        // move each particle as it falls
        const alive: Particle[] = [];
        for (const particle of this.particles) {
            particle.life -= elapsed;
            if (particle.life <= 0) {
                continue;
            }
            particle.speedY += gravity * elapsed;
            particle.x += particle.speedX * elapsed;
            particle.y += particle.speedY * elapsed;

            // glow cream, fading out at the end of its life
            const fade = Math.min(1, particle.life / (particle.span * 0.4));
            const size = 1.8;
            context.fillStyle = `rgba(255, 244, 222, ${(fade * 0.7).toFixed(3)})`;
            context.beginPath();
            context.arc(particle.x, particle.y, size, 0, Math.PI * 2);
            context.fill();
            context.fillStyle = `rgba(255, 180, 120, ${(fade * 0.1).toFixed(3)})`;
            context.beginPath();
            context.arc(particle.x, particle.y, size * 3, 0, Math.PI * 2);
            context.fill();

            // keep the live particles
            alive.push(particle);
        }
        this.particles = alive;

        // keep drawing while any particle lives
        if (alive.length > 0) {
            this.request();
        }
    }
}
