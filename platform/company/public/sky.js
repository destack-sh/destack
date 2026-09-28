// the seconds a clicked shooting star takes to burn out
const starLife = 1.2;
// the most device pixels per CSS pixel the sky renders at, since the stars are soft anyway
const largestScale = 2;

// the sky fragment shader: the same stars, clouds and shooting stars as the universe on destack.sh
const fragmentSource = `
precision mediump float;
uniform vec2 resolution;
uniform float scale;
uniform float time;
uniform vec2 pointer;
uniform float pull;
uniform vec3 star;

const vec3 space = vec3(0.031, 0.09, 0.137);
const vec3 rimColor = vec3(0.945, 0.918, 0.859);

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// return a sparse field of flat, crisp stars in a few sizes
vec3 starLayer(vec2 p, float cellSize, float density, float t) {
    vec2 cell = floor(p / cellSize);
    vec2 local = fract(p / cellSize) * cellSize;
    float seed = hash(cell);
    if (seed > density) {
        return vec3(0.0);
    }
    vec2 centre = (vec2(hash(cell + 1.3), hash(cell + 2.7)) * 0.7 + 0.15) * cellSize;
    float size = hash(cell + 5.1);
    float radius = size < 0.7 ? 0.7 : (size < 0.93 ? 1.1 : 1.6);
    float disc = 1.0 - smoothstep(radius - 0.5, radius + 0.5, length(local - centre));
    float alpha = mix(0.35, 0.9, hash(cell + 7.7)) * (0.7 + 0.3 * sin(t * (0.5 + hash(cell + 9.0) * 0.9) + seed * 40.0));

    // tint some stars in soft pastels
    float hue = hash(cell + 3.3);
    vec3 tint = hue < 0.18 ? vec3(0.98, 0.72, 0.84) : (hue < 0.36 ? vec3(0.66, 0.85, 1.0) : (hue < 0.5 ? vec3(1.0, 0.9, 0.62) : (hue < 0.6 ? vec3(0.7, 0.95, 0.84) : rimColor)));
    return mix(rimColor, tint, 0.8) * disc * alpha;
}

// return one shooting star's light at a point, one streak per period
vec3 streak(vec2 frag, vec2 size, float time, float period, float track) {
    float round = floor(time / period);
    float phase = fract(time / period) / 0.14;
    if (phase >= 1.0) {
        return vec3(0.0);
    }
    vec2 start = vec2(hash(vec2(round, 1.0 + track)), hash(vec2(round, 2.0 + track)) * 0.6) * size;
    vec2 direction = normalize(vec2(1.0, 0.3 + hash(vec2(round, 3.0 + track)) * 0.4));
    vec2 head = start + direction * phase * size.x * 0.7;
    vec2 relative = frag - head;
    float behind = dot(relative, -direction);
    float across = length(relative + direction * behind);
    float tail = step(0.0, behind) * exp(-behind / 45.0) * exp(-across * across / 0.8);
    return vec3(1.0, 0.97, 0.92) * tail * (1.0 - phase) * 0.9;
}

void main() {
    // work in CSS pixels from the top left
    vec2 size = resolution / scale;
    vec2 frag = vec2(gl_FragCoord.x, resolution.y - gl_FragCoord.y) / scale;

    // bend the stars away from the pointer like a lens
    vec2 sky = frag;
    vec2 toward = frag - pointer;
    sky += toward / max(length(toward), 1.0) * 22.0 * pull * exp(-dot(toward, toward) / 4200.0);

    // glow in slow clouds under two depths of drifting stars
    float falloff = smoothstep(0.25, 0.85, noise(frag * 0.0012 + vec2(time * 0.004, 0.0)));
    vec3 nebula = (vec3(0.09, 0.2, 0.27) - space) * falloff * 0.45;
    vec3 distant = starLayer(sky + vec2(time * 1.2, time * 0.3), 26.0, 0.14, time);
    vec3 close = starLayer(sky + 71.0 + vec2(time * 2.4, time * 0.6), 58.0, 0.16, time * 1.3) * 1.25;
    vec3 color = space + nebula + distant * 0.6 + close * 0.8;

    // cross the sky with a shooting star now and then
    color += streak(frag, size, time, 13.0, 1.0);

    // streak a shooting star away from where the reader clicked, flashing where it starts
    if (star.z >= 0.0 && star.z < ${starLife.toFixed(1)}) {
        float life = star.z / ${starLife.toFixed(1)};
        vec2 heading = normalize(vec2(-0.82, 0.57));
        vec2 head = star.xy + heading * star.z * 520.0;
        float tail = 120.0 * (1.0 - life);
        vec2 back = frag - head;
        float along = clamp(dot(back, -heading) / max(tail, 1.0), 0.0, 1.0);
        float across = length(back + heading * along * tail);
        float trail = exp(-across * across / 2.2) * (1.0 - along) * (1.0 - life);
        float flash = exp(-dot(frag - star.xy, frag - star.xy) / 40.0) * max(0.0, 1.0 - star.z * 5.0);
        color += vec3(1.0, 0.95, 0.85) * (trail * 1.4 + flash);
    }

    gl_FragColor = vec4(color, 1.0);
}
`;

// the vertex shader: one triangle that covers the screen
const vertexSource = `
attribute vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}
`;

// draw the sky behind the page, leaving the still star tile in place without WebGL
const canvas = document.querySelector("canvas.sky");
const gl = canvas.getContext("webgl", { antialias: false, alpha: false });
if (gl) {
    // compile and link the program
    const compile = (type, source) => {
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            throw new Error(`sky shader failed to compile: ${gl.getShaderInfoLog(shader)}`);
        }

        return shader;
    };
    const program = gl.createProgram();
    gl.attachShader(program, compile(gl.VERTEX_SHADER, vertexSource));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, fragmentSource));
    gl.linkProgram(program);
    gl.useProgram(program);

    // cover the screen with one triangle
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    const uniform = (name) => gl.getUniformLocation(program, name);
    const uniforms = {
        resolution: uniform("resolution"),
        scale: uniform("scale"),
        time: uniform("time"),
        pointer: uniform("pointer"),
        pull: uniform("pull"),
        star: uniform("star"),
    };

    // hold the pointer, how strongly it bends the stars, and the last clicked star
    const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const pointer = { x: -1000, y: -1000, pull: 0, target: 0 };
    let clicked = { x: 0, y: 0, at: -Infinity };
    let last = performance.now();

    // size the canvas to the window in device pixels
    const resize = () => {
        const scale = Math.min(window.devicePixelRatio, largestScale);
        canvas.width = Math.round(innerWidth * scale);
        canvas.height = Math.round(innerHeight * scale);
        gl.viewport(0, 0, canvas.width, canvas.height);
        gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
        gl.uniform1f(uniforms.scale, scale);
    };

    // draw one frame, easing the lens toward the pointer
    const draw = (now) => {
        const step = Math.min(0.05, (now - last) / 1000);
        last = now;
        pointer.pull += (pointer.target - pointer.pull) * Math.min(1, step * 6);
        gl.uniform1f(uniforms.time, isStill ? 0 : now / 1000);
        gl.uniform2f(uniforms.pointer, pointer.x, pointer.y);
        gl.uniform1f(uniforms.pull, isStill ? 0 : pointer.pull);
        gl.uniform3f(uniforms.star, clicked.x, clicked.y, (now - clicked.at) / 1000);
        gl.drawArrays(gl.TRIANGLES, 0, 3);
    };

    // animate while the page shows, or draw once for readers who prefer less motion
    const frame = (now) => {
        draw(now);
        if (!document.hidden) {
            requestAnimationFrame(frame);
        }
    };
    resize();
    canvas.classList.add("is-painted");
    if (isStill) {
        draw(0);
        window.addEventListener("resize", () => {
            resize();
            draw(0);
        });
    } else {
        window.addEventListener("resize", resize);
        document.addEventListener("visibilitychange", () => {
            if (!document.hidden) {
                requestAnimationFrame(frame);
            }
        });
        requestAnimationFrame(frame);
    }

    // follow the pointer, and send a shooting star from empty sky when clicked
    window.addEventListener("pointermove", (event) => {
        pointer.x = event.clientX;
        pointer.y = event.clientY;
        pointer.target = 1;
    });
    document.documentElement.addEventListener("pointerleave", () => {
        pointer.target = 0;
    });
    window.addEventListener("pointerdown", (event) => {
        if (!event.target.closest("a, button")) {
            clicked = { x: event.clientX, y: event.clientY, at: performance.now() };
        }
    });
}
