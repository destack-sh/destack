import img from "./vercel.svg";
import secondaryImg from "./vercel-mark.svg";
import badge from "./badge.svg";
import { buildImageLabel, buildImageManifest } from "./labels.ts";

const manifest = buildImageManifest({
    primary: img,
    secondary: secondaryImg,
    badge,
});

console.log(manifest, buildImageLabel("vercel"));
