/// The most particles alive at once.
const capacity = 400;
/// The pull of gravity on a spark, in CSS pixels per second squared.
const gravity = 260;

/// Draw light particles on a canvas: sparks that burst and fall, and motes that swirl down into a drain.
export class Sparks {
    /// The canvas the particles are drawn on.
    canvas: HTMLCanvasElement;
    /// The drawing context.
    context: CanvasRenderingContext2D;
    /// The particles' positions across, in CSS pixels.
    x: Float32Array;
    /// The particles' positions down, in CSS pixels.
    y: Float32Array;
    /// The particles' speeds across, in CSS pixels per second.
    speedX: Float32Array;
    /// The particles' speeds down, in CSS pixels per second.
    speedY: Float32Array;
    /// The particles' remaining lives, in seconds.
    life: Float32Array;
    /// The particles' full lives, in seconds.
    span: Float32Array;
    /// Whether each particle swirls toward the drain instead of falling.
    isMote: Uint8Array;
    /// The number of particles alive.
    count: number;
    /// The drain the motes swirl into, in CSS pixels.
    drain: { x: number; y: number };
    /// The pending animation frame, if any.
    frame: number | undefined;
    /// The time of the last frame in milliseconds.
    last: number;

    /// Create the particles on a canvas.
    constructor(canvas: HTMLCanvasElement) {
        this.canvas = canvas;
        this.context = canvas.getContext("2d")!;
        this.x = new Float32Array(capacity);
        this.y = new Float32Array(capacity);
        this.speedX = new Float32Array(capacity);
        this.speedY = new Float32Array(capacity);
        this.life = new Float32Array(capacity);
        this.span = new Float32Array(capacity);
        this.isMote = new Uint8Array(capacity);
        this.count = 0;
        this.drain = { x: 0, y: 0 };
        this.frame = undefined;
        this.last = 0;
    }

    /// Burst a spray of sparks from a point, flung up and out.
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
                false,
            );
        }
        this.request();
    }

    /// Scatter motes along a line that then swirl down into the drain.
    swirl(fromX: number, toX: number, y: number, count: number) {
        for (let index = 0; index < count; index++) {
            const x = fromX + Math.random() * (toX - fromX);
            this.add(x, y + Math.random() * 30, 0, 0, 1.6 + Math.random() * 1.2, true);
        }
        this.request();
    }

    /// Add one particle, replacing the oldest when full.
    add(x: number, y: number, speedX: number, speedY: number, life: number, isMote: boolean) {
        const index = this.count < capacity ? this.count++ : Math.floor(Math.random() * capacity);
        this.x[index] = x;
        this.y[index] = y;
        this.speedX[index] = speedX;
        this.speedY[index] = speedY;
        this.life[index] = life;
        this.span[index] = life;
        this.isMote[index] = isMote ? 1 : 0;
    }

    /// Schedule the next frame once.
    request() {
        if (this.frame === undefined) {
            this.last = performance.now();
            this.frame = requestAnimationFrame((now) => this.draw(now));
        }
    }

    /// Stop drawing.
    stop() {
        if (this.frame !== undefined) {
            cancelAnimationFrame(this.frame);
            this.frame = undefined;
        }
    }

    /// Move and draw every particle, and keep going while any are alive.
    draw(now: number) {
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

        // move each particle: sparks fall, motes spiral inward toward the drain
        let alive = 0;
        for (let index = 0; index < this.count; index++) {
            this.life[index] -= elapsed;
            if (this.life[index] <= 0) {
                continue;
            }
            if (this.isMote[index] === 1) {
                const towardX = this.drain.x - this.x[index];
                const towardY = this.drain.y - this.y[index];
                const distance = Math.hypot(towardX, towardY) + 1;
                const pull = 1600 + 90000 / distance;
                this.speedX[index] +=
                    ((towardX / distance) * pull - (towardY / distance) * pull * 0.6) * elapsed;
                this.speedY[index] +=
                    ((towardY / distance) * pull + (towardX / distance) * pull * 0.6) * elapsed;
                this.speedX[index] *= 0.93;
                this.speedY[index] *= 0.93;
                if (distance < 10) {
                    this.life[index] = 0;
                    continue;
                }
            } else {
                this.speedY[index] += gravity * elapsed;
            }
            this.x[index] += this.speedX[index] * elapsed;
            this.y[index] += this.speedY[index] * elapsed;

            // glow cream, fading out at the end of its life
            const fade = Math.min(1, this.life[index] / (this.span[index] * 0.4));
            const size = this.isMote[index] === 1 ? 1.4 : 1.8;
            context.fillStyle = `rgba(255, 244, 222, ${(fade * 0.9).toFixed(3)})`;
            context.beginPath();
            context.arc(this.x[index], this.y[index], size, 0, Math.PI * 2);
            context.fill();
            context.fillStyle = `rgba(255, 180, 120, ${(fade * 0.18).toFixed(3)})`;
            context.beginPath();
            context.arc(this.x[index], this.y[index], size * 3, 0, Math.PI * 2);
            context.fill();

            // keep the live particles packed at the front
            this.x[alive] = this.x[index];
            this.y[alive] = this.y[index];
            this.speedX[alive] = this.speedX[index];
            this.speedY[alive] = this.speedY[index];
            this.life[alive] = this.life[index];
            this.span[alive] = this.span[index];
            this.isMote[alive] = this.isMote[index];
            alive += 1;
        }
        this.count = alive;

        if (alive > 0) {
            this.request();
        }
    }
}
