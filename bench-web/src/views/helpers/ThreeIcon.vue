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
const FLOAT_AMP = 0.08;
const FLOAT_SPEED = 0.8;
const MOUSE_INF = 0.1;
const CAM_LERP = 0.05;

const canvasRef = ref<HTMLCanvasElement | null>(null);

/* three objects */
let scene: THREE.Scene,
  cam: THREE.PerspectiveCamera,
  ren: THREE.WebGLRenderer | null = null,
  clock: THREE.Clock,
  frame = 0;

const stack = new THREE.Group();
const layers: THREE.Mesh[] = [];
const baseY: number[] = [];

/* mouse-parallax state */
const mouse = reactive({ x: 0, y: 0 });
const camTgt = reactive({ x: 0, y: 0 });
const camCur = reactive({ x: 0, y: 0 });

/* ── helpers ───────────────────────────── */
function onMove(e: MouseEvent) {
  if (!canvasRef.value) return;
  const r = canvasRef.value.getBoundingClientRect();
  mouse.x = ((e.clientX - r.left) / r.width) * 2 - 1;
  mouse.y = -((e.clientY - r.top) / r.height) * 2 + 1;
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

  /* scene / clock */
  scene = new THREE.Scene();
  clock = new THREE.Clock();

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
  onResize();

  /* main loop */
  const loop = () => {
    const t = clock.getElapsedTime();

    layers.forEach((m, i) => {
      m.position.y = baseY[i] + Math.sin(t * FLOAT_SPEED * (1 + i * 0.2) + i) * FLOAT_AMP;
    });

    camCur.x += (camTgt.x - camCur.x) * CAM_LERP;
    camCur.y += (camTgt.y - camCur.y) * CAM_LERP;
    cam.lookAt(camCur.x, 0, camCur.y);

    ren!.render(scene, cam);
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
    (l.material as THREE.Material).dispose();
  });
  ren?.dispose();
});
</script>

<template>
  <canvas ref="canvasRef" />
</template>
