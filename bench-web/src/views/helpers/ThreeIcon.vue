<script lang="ts" setup>
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment";
import { RoundedBoxGeometry } from "three/examples/jsm/geometries/RoundedBoxGeometry.js";
import { onMounted, onBeforeUnmount, ref, reactive } from "vue";

const NUM_LAYERS = 3;
const W = 2.0;
const D = 2.0;
const H = 0.25;
const GAP = 0.35;
const ROUND_RADIUS = 0.1;
const ROUND_SEGMENTS = 4;
const FLOAT_AMP = 0.04;
const FLOAT_SPEED = 0.5;
const MOUSE_INF = 0.1;
const CAM_LERP = 0.05;
const HOVER_ROTATION = Math.PI / 6; // Target rotation on hover
const ROTATION_LERP = 0.1; // Speed of rotation interpolation

const canvasRef = ref<HTMLCanvasElement | null>(null);

/* three objects */
let scene: THREE.Scene,
  cam: THREE.PerspectiveCamera,
  ren: THREE.WebGLRenderer | null = null,
  clock: THREE.Clock,
  raycaster: THREE.Raycaster,
  frame = 0;

const stack = new THREE.Group();
const layers: THREE.Mesh[] = [];
const baseY: number[] = [];

/* mouse/hover state */
const mouse = reactive({ x: 0, y: 0 }); // Normalized device coordinates for raycasting/parallax
const camTgt = reactive({ x: 0, y: 0 }); // Target offset for camera parallax
const camCur = reactive({ x: 0, y: 0 }); // Current offset for camera parallax
let hoveredLayer: THREE.Mesh | null = null;

/* ── helpers ───────────────────────────── */
function onMove(e: MouseEvent) {
  if (!canvasRef.value) return;
  const r = canvasRef.value.getBoundingClientRect();
  // Update mouse NDC
  mouse.x = ((e.clientX - r.left) / r.width) * 2 - 1;
  mouse.y = -((e.clientY - r.top) / r.height) * 2 + 1;
  // Update camera parallax target
  camTgt.x = -mouse.x * MOUSE_INF;
  camTgt.y = -mouse.y * MOUSE_INF;
}
function onResize() {
  if (!canvasRef.value || !ren) return;
  const p = canvasRef.value.parentElement!;
  cam.aspect = p.clientWidth / p.clientHeight;
  cam.updateProjectionMatrix();
  ren.setSize(p.clientWidth, p.clientHeight);
  ren.setPixelRatio(Math.min(window.devicePixelRatio, 2));
}
/* ───────────────────────────────────────── */

onMounted(() => {
  if (!canvasRef.value) return;

  /* scene / clock / raycaster */
  scene = new THREE.Scene();
  clock = new THREE.Clock();
  raycaster = new THREE.Raycaster();

  /* camera */
  const p = canvasRef.value.parentElement!;
  cam = new THREE.PerspectiveCamera(38, p.clientWidth / p.clientHeight, 0.1, 100);
  cam.position.set(8, 8, 8);
  cam.lookAt(0, 0, 0);

  /* renderer */
  ren = new THREE.WebGLRenderer({ canvas: canvasRef.value, antialias: true, alpha: true });
  ren.setClearColor(0x000000, 0);
  ren.toneMapping = THREE.ACESFilmicToneMapping;
  ren.toneMappingExposure = 1.0;
  ren.outputColorSpace = THREE.SRGBColorSpace;

  /* enable soft shadows */
  ren.shadowMap.enabled = true;
  ren.shadowMap.type = THREE.PCFSoftShadowMap;

  /* environment map */
  const pmrem = new THREE.PMREMGenerator(ren);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  pmrem.dispose();

  /* lights */
  scene.add(new THREE.HemisphereLight(0xffffff, 0x555555, 0.5));

  /* geometry / material */
  const geom = new RoundedBoxGeometry(W, H, D, ROUND_SEGMENTS, ROUND_RADIUS);
  const mat = new THREE.MeshPhysicalMaterial({
    color: 0x101010,
    metalness: 0.0,
    roughness: 0.9,
    clearcoat: 0.3,
    clearcoatRoughness: 0.5,
    envMapIntensity: 1.2,
  });

  /* build stack */
  const topY = ((NUM_LAYERS - 1) * (H + GAP)) / 2;
  for (let i = 0; i < NUM_LAYERS; i++) {
    const mesh = new THREE.Mesh(geom, mat);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    mesh.position.y = topY - i * (H + GAP);
    baseY.push(mesh.position.y);
    layers.push(mesh);
    stack.add(mesh);
  }
  scene.add(stack);

  /* listeners */
  window.addEventListener("mousemove", onMove);
  window.addEventListener("resize", onResize);
  onResize(); // Initial size calculation

  /* main loop */
  const loop = () => {
    if (!ren) return; // Exit if renderer is disposed
    const t = clock.getElapsedTime();

    // 1. Raycasting: Find hovered layer
    raycaster.setFromCamera(new THREE.Vector2(mouse.x, mouse.y), cam);
    const intersects = raycaster.intersectObjects(layers);
    // Check if the first intersected object is one of our layers
    const firstIntersectedLayer = layers.find(l => intersects.length > 0 && l === intersects[0].object);
    hoveredLayer = firstIntersectedLayer ?? null;

    // 2. Update layers: position (float) and rotation (hover)
    layers.forEach((m, i) => {
      // Floating position
      m.position.y = baseY[i] + Math.sin(t * FLOAT_SPEED * (1 + i * 0.2) + i) * FLOAT_AMP;

      // Hover rotation (interpolate towards target)
      const targetRotationY = m === hoveredLayer ? HOVER_ROTATION : 0;
      // Use lerp for smooth transition in both directions
      m.rotation.y += (targetRotationY - m.rotation.y) * ROTATION_LERP;
    });

    // 3. Update camera: parallax effect
    camCur.x += (camTgt.x - camCur.x) * CAM_LERP;
    camCur.y += (camTgt.y - camCur.y) * CAM_LERP;
    // Look towards the center point offset by the interpolated mouse position
    cam.lookAt(camCur.x, 0, camCur.y);

    // 4. Render
    ren.render(scene, cam);
    frame = window.requestAnimationFrame(loop);
  };
  loop();
});

onBeforeUnmount(() => {
  window.cancelAnimationFrame(frame);
  window.removeEventListener("mousemove", onMove);
  window.removeEventListener("resize", onResize);
  layers.forEach((l) => {
    l.geometry.dispose();
    // Material is shared, dispose only once if needed, but safer to check
    if (l.material instanceof THREE.Material) {
      // Check if it's the last layer using the material before disposing
      // Or simply don't dispose shared materials here if they might be used elsewhere
      // For this component, disposing is likely fine.
      l.material.dispose();
    }
  });
  // Clear arrays
  layers.length = 0;
  baseY.length = 0;
  // Dispose renderer and scene resources
  ren?.dispose();
  scene?.clear(); // Remove objects, geometries, materials from scene if needed
  ren = null; // Allow garbage collection
});
</script>

<template>
  <canvas ref="canvasRef" />
</template>
