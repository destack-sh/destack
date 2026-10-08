# @destack/shader

`@destack/shader/<effect>` is Paper Shaders' effects as Solid components (`MeshGradient`, `Metaballs`, `GrainGradient`… with `colors`, `speed`, `frame`, `fit`, `scale`, `minPixelRatio`), and `Shader` is its `ShaderMount`.

```tsx
<Metaballs colors={[color.primary, color.accent]} colorBack={color.background} />; // theme tokens as colors
<Shader fragmentShader={glow} uniforms={{ u_color: color.primary }} />; // ShaderMount's fragmentShader
<Shader
    fragmentShader={glow}
    uniforms={{}}
    fallback={<img alt="" src="/glow.png" />}
    onFailure={report}
/>; // Paper Shaders has no fallback
```

## Motion

Effects follow the person's motion setting and show one still frame where it reduces motion.

```tsx
<Waves speed={0} frame={2400} />
```

## Mount

`onMount` receives the running mount, whose `frame` is the time the shader reads as `u_time`.

```tsx
<Shader
    fragmentShader={water}
    uniforms={{ u_movedAt: movedAt() }}
    onMount={(mount) => setMount(mount)}
/>
```
