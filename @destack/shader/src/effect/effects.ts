// generate with `bun run generate` from the shaders effect registry

import type * as angularBlur from "shaders/core/AngularBlur";
import type * as arc from "shaders/core/Arc";
import type * as ascii from "shaders/core/Ascii";
import type * as aurora from "shaders/core/Aurora";
import type * as barnDoors from "shaders/core/BarnDoors";
import type * as barShift from "shaders/core/BarShift";
import type * as beam from "shaders/core/Beam";
import type * as bend from "shaders/core/Bend";
import type * as blob from "shaders/core/Blob";
import type * as blockDissolve from "shaders/core/BlockDissolve";
import type * as blockNoise from "shaders/core/BlockNoise";
import type * as blueNoise from "shaders/core/BlueNoise";
import type * as blur from "shaders/core/Blur";
import type * as boids from "shaders/core/Boids";
import type * as bokehBlur from "shaders/core/BokehBlur";
import type * as brickPattern from "shaders/core/BrickPattern";
import type * as brightnessContrast from "shaders/core/BrightnessContrast";
import type * as brushedMetal from "shaders/core/BrushedMetal";
import type * as bulge from "shaders/core/Bulge";
import type * as carbonFiber from "shaders/core/CarbonFiber";
import type * as chalkboard from "shaders/core/Chalkboard";
import type * as channelBlur from "shaders/core/ChannelBlur";
import type * as checkerboard from "shaders/core/Checkerboard";
import type * as checkerWipe from "shaders/core/CheckerWipe";
import type * as chevron from "shaders/core/Chevron";
import type * as chromaFlow from "shaders/core/ChromaFlow";
import type * as chromaticAberration from "shaders/core/ChromaticAberration";
import type * as chrome from "shaders/core/Chrome";
import type * as circle from "shaders/core/Circle";
import type * as colorWheel from "shaders/core/ColorWheel";
import type * as compressionArtifacts from "shaders/core/CompressionArtifacts";
import type * as concentricSpin from "shaders/core/ConcentricSpin";
import type * as conicGradient from "shaders/core/ConicGradient";
import type * as contourLines from "shaders/core/ContourLines";
import type * as cornerPin from "shaders/core/CornerPin";
import type * as crescent from "shaders/core/Crescent";
import type * as cross from "shaders/core/Cross";
import type * as cRTScreen from "shaders/core/CRTScreen";
import type * as crystal from "shaders/core/Crystal";
import type * as curlNoise from "shaders/core/CurlNoise";
import type * as cursorRipples from "shaders/core/CursorRipples";
import type * as cursorTrail from "shaders/core/CursorTrail";
import type * as dataMosh from "shaders/core/DataMosh";
import type * as diamondGradient from "shaders/core/DiamondGradient";
import type * as diamondWipe from "shaders/core/DiamondWipe";
import type * as diffuseBlur from "shaders/core/DiffuseBlur";
import type * as displacementMap from "shaders/core/DisplacementMap";
import type * as dither from "shaders/core/Dither";
import type * as dotGrid from "shaders/core/DotGrid";
import type * as dropShadow from "shaders/core/DropShadow";
import type * as duotone from "shaders/core/Duotone";
import type * as ellipse from "shaders/core/Ellipse";
import type * as emboss from "shaders/core/Emboss";
import type * as engraving from "shaders/core/Engraving";
import type * as erosionNoise from "shaders/core/ErosionNoise";
import type * as exposure from "shaders/core/Exposure";
import type * as fallingLines from "shaders/core/FallingLines";
import type * as filmGrain from "shaders/core/FilmGrain";
import type * as filmStock from "shaders/core/FilmStock";
import type * as flip from "shaders/core/Flip";
import type * as floatingParticles from "shaders/core/FloatingParticles";
import type * as flower from "shaders/core/Flower";
import type * as flowField from "shaders/core/FlowField";
import type * as flowingGradient from "shaders/core/FlowingGradient";
import type * as flutedGlass from "shaders/core/FlutedGlass";
import type * as fog from "shaders/core/Fog";
import type * as form3D from "shaders/core/Form3D";
import type * as fractalNoise from "shaders/core/FractalNoise";
import type * as frost from "shaders/core/Frost";
import type * as gaborNoise from "shaders/core/GaborNoise";
import type * as glass from "shaders/core/Glass";
import type * as glassTiles from "shaders/core/GlassTiles";
import type * as glitch from "shaders/core/Glitch";
import type * as glow from "shaders/core/Glow";
import type * as godrays from "shaders/core/Godrays";
import type * as goo from "shaders/core/Goo";
import type * as gradientMap from "shaders/core/GradientMap";
import type * as grayscale from "shaders/core/Grayscale";
import type * as grid from "shaders/core/Grid";
import type * as gridDistortion from "shaders/core/GridDistortion";
import type * as group from "shaders/core/Group";
import type * as halftone from "shaders/core/Halftone";
import type * as heart from "shaders/core/Heart";
import type * as heatmap from "shaders/core/Heatmap";
import type * as hexGrid from "shaders/core/HexGrid";
import type * as hologram from "shaders/core/Hologram";
import type * as holographic from "shaders/core/Holographic";
import type * as hTMLInCanvas from "shaders/core/HTMLInCanvas";
import type * as hueShift from "shaders/core/HueShift";
import type * as imageTexture from "shaders/core/ImageTexture";
import type * as inkFlow from "shaders/core/InkFlow";
import type * as invert from "shaders/core/Invert";
import type * as irisWipe from "shaders/core/IrisWipe";
import type * as irradiance from "shaders/core/Irradiance";
import type * as isometricCubes from "shaders/core/IsometricCubes";
import type * as kaleidoscope from "shaders/core/Kaleidoscope";
import type * as keyFrames from "shaders/core/KeyFrames";
import type * as lensDistortion from "shaders/core/LensDistortion";
import type * as lensFlare from "shaders/core/LensFlare";
import type * as lightEdge from "shaders/core/LightEdge";
import type * as lightLeak from "shaders/core/LightLeak";
import type * as line from "shaders/core/Line";
import type * as linearBlur from "shaders/core/LinearBlur";
import type * as linearGradient from "shaders/core/LinearGradient";
import type * as linearWipe from "shaders/core/LinearWipe";
import type * as liquidMetal from "shaders/core/LiquidMetal";
import type * as liquify from "shaders/core/Liquify";
import type * as magneticFilings from "shaders/core/MagneticFilings";
import type * as marble from "shaders/core/Marble";
import type * as meshGradient from "shaders/core/MeshGradient";
import type * as mirror from "shaders/core/Mirror";
import type * as multiPointGradient from "shaders/core/MultiPointGradient";
import type * as nebula from "shaders/core/Nebula";
import type * as neon from "shaders/core/Neon";
import type * as noiseDissolve from "shaders/core/NoiseDissolve";
import type * as objectTracker from "shaders/core/ObjectTracker";
import type * as obsidian from "shaders/core/Obsidian";
import type * as pagePeel from "shaders/core/PagePeel";
import type * as paper from "shaders/core/Paper";
import type * as parallelogram from "shaders/core/Parallelogram";
import type * as particleField from "shaders/core/ParticleField";
import type * as particleFlow from "shaders/core/ParticleFlow";
import type * as particles from "shaders/core/Particles";
import type * as perlinNoise from "shaders/core/PerlinNoise";
import type * as perspective from "shaders/core/Perspective";
import type * as pixelate from "shaders/core/Pixelate";
import type * as pixelSort from "shaders/core/PixelSort";
import type * as pixelThrow from "shaders/core/PixelThrow";
import type * as plasma from "shaders/core/Plasma";
import type * as plastic from "shaders/core/Plastic";
import type * as polarCoordinates from "shaders/core/PolarCoordinates";
import type * as polygon from "shaders/core/Polygon";
import type * as posterize from "shaders/core/Posterize";
import type * as prism from "shaders/core/Prism";
import type * as progressiveBlur from "shaders/core/ProgressiveBlur";
import type * as radialGradient from "shaders/core/RadialGradient";
import type * as radialWipe from "shaders/core/RadialWipe";
import type * as randomBars from "shaders/core/RandomBars";
import type * as reactionDiffusion from "shaders/core/ReactionDiffusion";
import type * as rectangularCoordinates from "shaders/core/RectangularCoordinates";
import type * as reflectivePlane from "shaders/core/ReflectivePlane";
import type * as repeater from "shaders/core/Repeater";
import type * as ring from "shaders/core/Ring";
import type * as ripples from "shaders/core/Ripples";
import type * as rippleWipe from "shaders/core/RippleWipe";
import type * as roundedRect from "shaders/core/RoundedRect";
import type * as saturation from "shaders/core/Saturation";
import type * as scratches from "shaders/core/Scratches";
import type * as sharpness from "shaders/core/Sharpness";
import type * as shatter from "shaders/core/Shatter";
import type * as simplexNoise from "shaders/core/SimplexNoise";
import type * as sineWave from "shaders/core/SineWave";
import type * as sliceWipe from "shaders/core/SliceWipe";
import type * as smoke from "shaders/core/Smoke";
import type * as smokeFill from "shaders/core/SmokeFill";
import type * as smokeFlow from "shaders/core/SmokeFlow";
import type * as solarize from "shaders/core/Solarize";
import type * as solidColor from "shaders/core/SolidColor";
import type * as sparkle from "shaders/core/Sparkle";
import type * as spherize from "shaders/core/Spherize";
import type * as spiral from "shaders/core/Spiral";
import type * as star from "shaders/core/Star";
import type * as stone from "shaders/core/Stone";
import type * as strands from "shaders/core/Strands";
import type * as stretch from "shaders/core/Stretch";
import type * as stripes from "shaders/core/Stripes";
import type * as studioBackground from "shaders/core/StudioBackground";
import type * as sunBurst from "shaders/core/SunBurst";
import type * as surface3D from "shaders/core/Surface3D";
import type * as swirl from "shaders/core/Swirl";
import type * as teardrop from "shaders/core/Teardrop";
import type * as text from "shaders/core/Text";
import type * as thinFilm from "shaders/core/ThinFilm";
import type * as tiltShift from "shaders/core/TiltShift";
import type * as timeTrail from "shaders/core/TimeTrail";
import type * as tint from "shaders/core/Tint";
import type * as trapezoid from "shaders/core/Trapezoid";
import type * as triangularGrid from "shaders/core/TriangularGrid";
import type * as tritone from "shaders/core/Tritone";
import type * as truchet from "shaders/core/Truchet";
import type * as twirl from "shaders/core/Twirl";
import type * as venetianBlinds from "shaders/core/VenetianBlinds";
import type * as vesica from "shaders/core/Vesica";
import type * as vHS from "shaders/core/VHS";
import type * as vibrance from "shaders/core/Vibrance";
import type * as videoTexture from "shaders/core/VideoTexture";
import type * as vignette from "shaders/core/Vignette";
import type * as voronoi from "shaders/core/Voronoi";
import type * as voxels from "shaders/core/Voxels";
import type * as water from "shaders/core/Water";
import type * as watercolor from "shaders/core/Watercolor";
import type * as waveDistortion from "shaders/core/WaveDistortion";
import type * as waveform from "shaders/core/Waveform";
import type * as waveletNoise from "shaders/core/WaveletNoise";
import type * as weave from "shaders/core/Weave";
import type * as webcamTexture from "shaders/core/WebcamTexture";
import type * as wool from "shaders/core/Wool";
import type * as worleyNoise from "shaders/core/WorleyNoise";
import type * as zoomBlur from "shaders/core/ZoomBlur";
import { defineEffect } from "./effect.tsx";

/** Radial motion blur rotating around a center point. */
export const AngularBlur = defineEffect<angularBlur.ComponentProps>("AngularBlur");

/** Pie sector (arc wedge) with adjustable radius and aperture angle. */
export const Arc = defineEffect<arc.ComponentProps>("Arc");

/** Convert imagery to ASCII character art. */
export const Ascii = defineEffect<ascii.ComponentProps>("Ascii");

/** Mesmerizing aurora borealis with layered curtains, vertical rays, and flowing light. */
export const Aurora = defineEffect<aurora.ComponentProps>("Aurora");

/** Split the content along a center line and wipe outward in both directions. */
export const BarnDoors = defineEffect<barnDoors.ComponentProps>("BarnDoors");

/** Slices content into parallel bars, each offset independently for a fractured or glitch-like effect. */
export const BarShift = defineEffect<barShift.ComponentProps>("BarShift");

/** A beam of light from one point to another. */
export const Beam = defineEffect<beam.ComponentProps>("Beam");

/** Bends the ends of the frame toward you like a curved display — content at the edges swells closer under real perspective, or curls away when the strength goes negative. */
export const Bend = defineEffect<bend.ComponentProps>("Bend");

/** Organic animated blob with 3D lighting and gradients. */
export const Blob = defineEffect<blob.ComponentProps>("Blob");

/** Dissolve the content away as a grid of blocks vanishing in random order. */
export const BlockDissolve = defineEffect<blockDissolve.ComponentProps>("BlockDissolve");

/** Blocky value noise with soft cells that morph over time. */
export const BlockNoise = defineEffect<blockNoise.ComponentProps>("BlockNoise");

/** High-frequency blue noise — even, grainy speckle ideal for dithering. */
export const BlueNoise = defineEffect<blueNoise.ComponentProps>("BlueNoise");

/** A simple Gaussian blur effect. */
export const Blur = defineEffect<blur.ComponentProps>("Blur");

/** A living murmuration of hundreds of flocking agents drawn as crisp arrows, streaks, dots or glowing comets — separation, alignment and cohesion drive fluid, splitting-and-merging ribbons, agents flash toward an excited color when scattered, and the cursor acts as an attractor or a predator. */
export const Boids = defineEffect<boids.ComponentProps>("Boids");

/** Photographic lens blur where bright highlights bloom into aperture-shaped discs. */
export const BokehBlur = defineEffect<bokehBlur.ComponentProps>("BokehBlur");

/** Classic brick wall pattern with alternating rows and mortar gaps. */
export const BrickPattern = defineEffect<brickPattern.ComponentProps>("BrickPattern");

/** Adjust brightness and contrast of the image. */
export const BrightnessContrast =
    defineEffect<brightnessContrast.ComponentProps>("BrightnessContrast");

/** Photorealistic brushed metal — a satin anodised surface combed with fine directional grain that smears the reflected studio and key light into the long anisotropic streaks of real brushed aluminium, steel, gold or copper. */
export const BrushedMetal = defineEffect<brushedMetal.ComponentProps>("BrushedMetal");

/** Magnify or pinch content around a center point. */
export const Bulge = defineEffect<bulge.ComponentProps>("Bulge");

/** Photorealistic woven carbon fibre — interlaced tows whose anisotropic sheen flips ninety degrees cell to cell, raised into a quilted weave and finished with a glossy clearcoat that mirrors the studio. */
export const CarbonFiber = defineEffect<carbonFiber.ComponentProps>("CarbonFiber");

/** Renders content as a chalk drawing on a blackboard, with edge strokes and cross-hatch shading. */
export const Chalkboard = defineEffect<chalkboard.ComponentProps>("Chalkboard");

/** Independent blur for red, green, and blue channels. */
export const ChannelBlur = defineEffect<channelBlur.ComponentProps>("ChannelBlur");

/** Classic checkerboard pattern with two alternating colors. */
export const Checkerboard = defineEffect<checkerboard.ComponentProps>("Checkerboard");

/** Wipe the content away as a checkerboard of fading squares. */
export const CheckerWipe = defineEffect<checkerWipe.ComponentProps>("CheckerWipe");

/** Animated chevron / zigzag stripe pattern. */
export const Chevron = defineEffect<chevron.ComponentProps>("Chevron");

/** Interactive liquid flow effect that follows your cursor. */
export const ChromaFlow = defineEffect<chromaFlow.ComponentProps>("ChromaFlow");

/** Separate RGB channels for a prismatic distortion effect. */
export const ChromaticAberration =
    defineEffect<chromaticAberration.ComponentProps>("ChromaticAberration");

/** Studio-lit mirror chrome — a precision-machined shape with a controllable polished bevel, softly convex faces and a photographic studio reflected in them: one huge frontal softbox, black flags, and thin warm/cool strip lights that fringe amber and ice-blue exactly where the reflections break. */
export const Chrome = defineEffect<chrome.ComponentProps>("Chrome");

/** Generate a circle with adjustable size and softness. */
export const Circle = defineEffect<circle.ComponentProps>("Circle");

/** A directional gradient that smoothly cycles through rainbow colors or a custom set of three colors. */
export const ColorWheel = defineEffect<colorWheel.ComponentProps>("ColorWheel");

/** Simulates lossy JPEG compression — 8×8 DCT block quantization, blockiness, ringing and color bleed. */
export const CompressionArtifacts =
    defineEffect<compressionArtifacts.ComponentProps>("CompressionArtifacts");

/** Concentric rings that each rotate the underlying image by different amounts. */
export const ConcentricSpin = defineEffect<concentricSpin.ComponentProps>("ConcentricSpin");

/** Colors sweep in a full circle around a center point, like a color wheel. */
export const ConicGradient = defineEffect<conicGradient.ComponentProps>("ConicGradient");

/** Draw topographical contour lines based on luminance or alpha. */
export const ContourLines = defineEffect<contourLines.ComponentProps>("ContourLines");

/** Pin each corner of the content to an arbitrary position for a free perspective warp. */
export const CornerPin = defineEffect<cornerPin.ComponentProps>("CornerPin");

/** Crescent moon shape — an outer circle with an inner circle subtracted. */
export const Crescent = defineEffect<crescent.ComponentProps>("Crescent");

/** Plus / cross shape with adjustable arm length, width, and rounding. */
export const Cross = defineEffect<cross.ComponentProps>("Cross");

/** Retro CRT monitor simulation with scanlines. */
export const CRTScreen = defineEffect<cRTScreen.ComponentProps>("CRTScreen");

/** Diamond-like crystal lens with faceted refraction. */
export const Crystal = defineEffect<crystal.ComponentProps>("Crystal");

/** Swirling divergence-free flow field that drifts over time. */
export const CurlNoise = defineEffect<curlNoise.ComponentProps>("CurlNoise");

/** Fluid-like ripple distortion. */
export const CursorRipples = defineEffect<cursorRipples.ComponentProps>("CursorRipples");

/** Animated trail effect that tracks cursor movement. */
export const CursorTrail = defineEffect<cursorTrail.ComponentProps>("CursorTrail");

/** Corrupted-codec motion smearing — macroblocks lock in place and drag their pixels across the frame in liquid trails, each recovering on its own clock, like a video stream that lost its keyframes. */
export const DataMosh = defineEffect<dataMosh.ComponentProps>("DataMosh");

/** Diamond-shaped gradient radiating from a center point using Manhattan distance. */
export const DiamondGradient = defineEffect<diamondGradient.ComponentProps>("DiamondGradient");

/** Wipe the content away through a lattice of growing diamonds. */
export const DiamondWipe = defineEffect<diamondWipe.ComponentProps>("DiamondWipe");

/** Grain-like pixel displacement at random. */
export const DiffuseBlur = defineEffect<diffuseBlur.ComponentProps>("DiffuseBlur");

/** Distorts child content using another layer's pixels as a displacement map. */
export const DisplacementMap = defineEffect<displacementMap.ComponentProps>("DisplacementMap");

/** Dithering effect with multiple pattern options. */
export const Dither = defineEffect<dither.ComponentProps>("Dither");

/** Grid of dots with optional twinkling animation. */
export const DotGrid = defineEffect<dotGrid.ComponentProps>("DotGrid");

/** Adds a soft shadow behind the child content based on its alpha silhouette. */
export const DropShadow = defineEffect<dropShadow.ComponentProps>("DropShadow");

/** Map colors to two tones based on luminance. */
export const Duotone = defineEffect<duotone.ComponentProps>("Duotone");

/** Ellipse with independently adjustable horizontal and vertical radii. */
export const Ellipse = defineEffect<ellipse.ComponentProps>("Ellipse");

/** Embossed / debossed relief shading on top of child content, driven by a custom shape. */
export const Emboss = defineEffect<emboss.ComponentProps>("Emboss");

/** Copper-plate line engraving — the image is redrawn as flowing line work whose weight swells with darkness, lines displaced by the form like a banknote portrait: a single plate, cross-hatched shadow plates, or one continuous spiral cut. */
export const Engraving = defineEffect<engraving.ComponentProps>("Engraving");

/** Branching, hydraulic-erosion ridges carved into noise. */
export const ErosionNoise = defineEffect<erosionNoise.ComponentProps>("ErosionNoise");

/** Multiplicative exposure (gain) on the child. */
export const Exposure = defineEffect<exposure.ComponentProps>("Exposure");

/** Directional falling lines with a leading-to-trailing color fade. */
export const FallingLines = defineEffect<fallingLines.ComponentProps>("FallingLines");

/** Analog film grain texture overlay, weighted toward darker areas. */
export const FilmGrain = defineEffect<filmGrain.ComponentProps>("FilmGrain");

/** Real analog film color from measured film-emulation LUTs — ten classic stock looks with emulsion halation and projector gate weave. */
export const FilmStock = defineEffect<filmStock.ComponentProps>("FilmStock");

/** Mirror content horizontally, vertically, or both. */
export const Flip = defineEffect<flip.ComponentProps>("Flip");

/** Drifting, twinkling motes — thousands of real simulated particles floating in a shared heading with per-particle wander and variance, and a cursor that stirs the field with a gust that settles back into the drift. */
export const FloatingParticles =
    defineEffect<floatingParticles.ComponentProps>("FloatingParticles");

/** Petal shape with N lobes and adjustable inner-to-outer radius ratio. */
export const Flower = defineEffect<flower.ComponentProps>("Flower");

/** Fluid-like distortion with constant smooth motion. */
export const FlowField = defineEffect<flowField.ComponentProps>("FlowField");

/** Liquid silk gradient with organic flowing color bands. */
export const FlowingGradient = defineEffect<flowingGradient.ComponentProps>("FlowingGradient");

/** Full-screen fluted glass effect — refracts content through repeating cylindrical bars. */
export const FlutedGlass = defineEffect<flutedGlass.ComponentProps>("FlutedGlass");

/** Fog that fills the screen and interacts with the mouse. */
export const Fog = defineEffect<fog.ComponentProps>("Fog");

/** Wraps child content onto a 3D raymarched shape with lighting. */
export const Form3D = defineEffect<form3D.ComponentProps>("Form3D");

/** Multi-octave fractal Brownian motion noise texture with true noise evolution. */
export const FractalNoise = defineEffect<fractalNoise.ComponentProps>("FractalNoise");

/** Photoreal frozen ice — thickness-driven subsurface scattering that reads as a solid block, with fine frost crystals creeping in from the edges. */
export const Frost = defineEffect<frost.ComponentProps>("Frost");

/** Oriented sine-grain noise with a fingerprint-like flow. */
export const GaborNoise = defineEffect<gaborNoise.ComponentProps>("GaborNoise");

/** Optically realistic glass lens driven in a custom shape. */
export const Glass = defineEffect<glass.ComponentProps>("Glass");

/** Refraction-like distortion in a tile grid pattern. */
export const GlassTiles = defineEffect<glassTiles.ComponentProps>("GlassTiles");

/** Digital glitch that melts pixels and distorts colors. */
export const Glitch = defineEffect<glitch.ComponentProps>("Glitch");

/** Soft glow effect with adjustable intensity. */
export const Glow = defineEffect<glow.ComponentProps>("Glow");

/** Volumetric light rays emanating from a point. */
export const Godrays = defineEffect<godrays.ComponentProps>("Godrays");

/** Photoreal wet liquid — animated 3D metaball blobs that merge and bulge to loosely form the shape, with sliding wet highlights, a clearcoat rim and a translucent body. */
export const Goo = defineEffect<goo.ComponentProps>("Goo");

/** Maps source luminance through an animated color gradient (Photoshop-style gradient map). */
export const GradientMap = defineEffect<gradientMap.ComponentProps>("GradientMap");

/** Convert colors to black and white. */
export const Grayscale = defineEffect<grayscale.ComponentProps>("Grayscale");

/** Simple grid lines pattern with adjustable thickness and rotation. */
export const Grid = defineEffect<grid.ComponentProps>("Grid");

/** Interactive grid distortion controlled by mouse position. */
export const GridDistortion = defineEffect<gridDistortion.ComponentProps>("GridDistortion");

/** Container for organizing and composing child effects — supports flex-like flow layout (column/row stacking via the flow prop). */
export const Group = defineEffect<group.ComponentProps>("Group");

/** Halftone dot pattern effect for printing aesthetics. */
export const Halftone = defineEffect<halftone.ComponentProps>("Halftone");

/** Heart shape with adjustable size. */
export const Heart = defineEffect<heart.ComponentProps>("Heart");

/** Thermal-camera heat flowing through any 2D, SVG, or 3D shape. */
export const Heatmap = defineEffect<heatmap.ComponentProps>("Heatmap");

/** Honeycomb hexagonal grid pattern. */
export const HexGrid = defineEffect<hexGrid.ComponentProps>("HexGrid");

/** Volumetric sci-fi hologram — a translucent emissive projection of the shape with fresnel-lit edges, depth scan-lines that wrap 3D forms, CRT scanlines, flicker and beam wobble. */
export const Hologram = defineEffect<hologram.ComponentProps>("Hologram");

/** Iridescent holographic foil sticker with animated rainbow sheen and glitter flakes. */
export const Holographic = defineEffect<holographic.ComponentProps>("Holographic");

/** Render live HTML/DOM content as a WebGPU texture layer via the html-in-canvas API. */
export const HTMLInCanvas = defineEffect<hTMLInCanvas.ComponentProps>("HTMLInCanvas");

/** Rotate hue around the color wheel. */
export const HueShift = defineEffect<hueShift.ComponentProps>("HueShift");

/** Display an image with customizable object-fit modes. */
export const ImageTexture = defineEffect<imageTexture.ComponentProps>("ImageTexture");

/** Drag to paint swirling ribbons of ink through a real fluid field — eddies keep evolving after you let go. */
export const InkFlow = defineEffect<inkFlow.ComponentProps>("InkFlow");

/** Invert RGB colors while preserving alpha. */
export const Invert = defineEffect<invert.ComponentProps>("Invert");

/** Reveal through an expanding circle growing from a center point. */
export const IrisWipe = defineEffect<irisWipe.ComponentProps>("IrisWipe");

/** Photorealistic light spilling around the edge of any 2D, SVG, or 3D shape — any number of movable colored point lights strike the silhouette and the lit edges irradiate their surroundings, gathered in a compute pass with real cast shadows, mixing where they meet and burning to white at the rim. */
export const Irradiance = defineEffect<irradiance.ComponentProps>("Irradiance");

/** Isometric tumbling-blocks tiling — a 3D cube illusion (rhombille pattern). */
export const IsometricCubes = defineEffect<isometricCubes.ComponentProps>("IsometricCubes");

/** Create a kaleidoscope effect with radial mirrored segments. */
export const Kaleidoscope = defineEffect<kaleidoscope.ComponentProps>("Kaleidoscope");

/** Motion-tracking overlay — trackers hunt and follow features in the content, dropping keyframe diamonds behind them along their motion paths, like a compositor mid-track. */
export const KeyFrames = defineEffect<keyFrames.ComponentProps>("KeyFrames");

/** Split content into shifting chromatic layers with barrel or pincushion lens warp. */
export const LensDistortion = defineEffect<lensDistortion.ComponentProps>("LensDistortion");

/** Realistic camera lens flare with artifacts. */
export const LensFlare = defineEffect<lensFlare.ComponentProps>("LensFlare");

/** Glowing, pulsing light racing around the edge of any 2D, SVG, or 3D shape. */
export const LightEdge = defineEffect<lightEdge.ComponentProps>("LightEdge");

/** Photorealistic film light leak — warm overexposed light bleeding in from a draggable anchor point, with streak bands, chromatic fringing, and slow breathing that evolves over time. */
export const LightLeak = defineEffect<lightLeak.ComponentProps>("LightLeak");

/** Draw a straight line between two points with color, thickness, and solid, dashed, or dotted styles. */
export const Line = defineEffect<line.ComponentProps>("Line");

/** Directional motion blur in a specific angle. */
export const LinearBlur = defineEffect<linearBlur.ComponentProps>("LinearBlur");

/** Create smooth linear color gradients. */
export const LinearGradient = defineEffect<linearGradient.ComponentProps>("LinearGradient");

/** Wipe the content away along a straight edge with a soft feathered transition. */
export const LinearWipe = defineEffect<linearWipe.ComponentProps>("LinearWipe");

/** Flowing liquid chrome — a molten reflective surface that wraps the shape, sweeping a procedural studio reflection across animated folds with crisp speculars and prismatic edges. */
export const LiquidMetal = defineEffect<liquidMetal.ComponentProps>("LiquidMetal");

/** Liquid-like interactive deformation effect. */
export const Liquify = defineEffect<liquify.ComponentProps>("Liquify");

/** Thousands of tiny iron filings scattered on paper that swing to align with a magnetic field around the cursor, tracing out the field lines — a dipole whose axis follows the cursor's motion draws the classic two-lobed swirl, and every sweep of the magnet sends a glowing wave of filings flipping and settling. */
export const MagneticFilings = defineEffect<magneticFilings.ComponentProps>("MagneticFilings");

/** Classic marble swirl and vein texture using noise-warped sine waves. */
export const Marble = defineEffect<marble.ComponentProps>("Marble");

/** Flowing mesh gradient of soft drifting color swaths whose seams wrap through the palette. */
export const MeshGradient = defineEffect<meshGradient.ComponentProps>("MeshGradient");

/** Mirror content across a line defined by center point and angle. */
export const Mirror = defineEffect<mirror.ComponentProps>("Mirror");

/** Five individually placed color points blended together by proximity — drag each point to shape the gradient. */
export const MultiPointGradient =
    defineEffect<multiPointGradient.ComponentProps>("MultiPointGradient");

/** A volumetric gas nebula sealed inside polished glass; billowing emission clouds with hollow dark cavities and hot glowing cores, star fields drifting at real depth behind the gas, all refracted through the curved walls of the shape and dressed with studio reflections. */
export const Nebula = defineEffect<nebula.ComponentProps>("Nebula");

/** Photorealistic neon tube / 3D pipe effect driven by a custom shape. */
export const Neon = defineEffect<neon.ComponentProps>("Neon");

/** Dissolve the content away through an organic noise pattern. */
export const NoiseDissolve = defineEffect<noiseDissolve.ComponentProps>("NoiseDissolve");

/** Computer-vision style object detection overlay — draws bounding boxes and labels around detected regions of the content below, with grid, quadtree and mosaic layouts. */
export const ObjectTracker = defineEffect<objectTracker.ComponentProps>("ObjectTracker");

/** Dark tinted glass whose faces stay near-black while every surface turning away from the viewer ignites with a flowing iridescent gradient — vivid color living only on the oblique walls and bevels. */
export const Obsidian = defineEffect<obsidian.ComponentProps>("Obsidian");

/** Curl the content up from a corner like a peeling page. */
export const PagePeel = defineEffect<pagePeel.ComponentProps>("PagePeel");

/** Applies realistic paper grain and surface roughness to child content. */
export const Paper = defineEffect<paper.ComponentProps>("Paper");

/** Parallelogram with adjustable width, height and skew. */
export const Parallelogram = defineEffect<parallelogram.ComponentProps>("Parallelogram");

/** Explodes the child content into a breathing 3D field of particles — bright areas float toward you and dark areas recede, the whole relief can be orbited with the camera rotations, and the cursor physically throws particles that spring back home. */
export const ParticleField = defineEffect<particleField.ComponentProps>("ParticleField");

/** Thousands of drifting dust particles carried by a real incompressible fluid field the cursor stirs — drag to plow a wake and the particles ride the eddies and vortices it leaves behind, while a gentle ambient breeze keeps the whole scene breathing on its own. */
export const ParticleFlow = defineEffect<particleFlow.ComponentProps>("ParticleFlow");

/** A swarm of simulated particles that settles into the shape, filling it evenly — flat, SVG or true 3D volumes — scattering from the cursor (or chasing it), tumbling when the shape moves, and always drifting back home. */
export const Particles = defineEffect<particles.ComponentProps>("Particles");

/** Smooth gradient noise that morphs over time. */
export const PerlinNoise = defineEffect<perlinNoise.ComponentProps>("PerlinNoise");

/** Rotate the plane in 3D space with pan and tilt. */
export const Perspective = defineEffect<perspective.ComponentProps>("Perspective");

/** Pixelation effect with adjustable cell size. */
export const Pixelate = defineEffect<pixelate.ComponentProps>("Pixelate");

/** Pixels sort by brightness around the cursor and keep their sorted position, optionally decaying back over time. */
export const PixelSort = defineEffect<pixelSort.ComponentProps>("PixelSort");

/** Throws pixels along the cursor's path like a fluid — brighter (or redder/greener/bluer) pixels are flung farther, then settle back with friction. */
export const PixelThrow = defineEffect<pixelThrow.ComponentProps>("PixelThrow");

/** Animated effect of glowing plasma. */
export const Plasma = defineEffect<plasma.ComponentProps>("Plasma");

/** Glossy molded plastic with photorealistic studio reflections, driven in a custom shape. */
export const Plastic = defineEffect<plastic.ComponentProps>("Plastic");

/** Convert rectangular coordinates to polar space. */
export const PolarCoordinates = defineEffect<polarCoordinates.ComponentProps>("PolarCoordinates");

/** Regular polygon with adjustable sides and corner rounding. */
export const Polygon = defineEffect<polygon.ComponentProps>("Polygon");

/** Reduce color depth to create a poster effect. */
export const Posterize = defineEffect<posterize.ComponentProps>("Posterize");

/** A beam of light that fans out and splits into a slowly-rotating rainbow past a controllable point. */
export const Prism = defineEffect<prism.ComponentProps>("Prism");

/** Blur that increases progressively in one direction. */
export const ProgressiveBlur = defineEffect<progressiveBlur.ComponentProps>("ProgressiveBlur");

/** Radial gradient radiating from a center point. */
export const RadialGradient = defineEffect<radialGradient.ComponentProps>("RadialGradient");

/** Sweep the content away in a clock-hand arc around a center point. */
export const RadialWipe = defineEffect<radialWipe.ComponentProps>("RadialWipe");

/** Wipe the content away as parallel bars vanishing in random order. */
export const RandomBars = defineEffect<randomBars.ComponentProps>("RandomBars");

/** A living Gray-Scott reaction-diffusion pattern that fills the layer and blooms wherever you drag the cursor. */
export const ReactionDiffusion =
    defineEffect<reactionDiffusion.ComponentProps>("ReactionDiffusion");

/** Convert polar coordinates back to rectangular space. */
export const RectangularCoordinates =
    defineEffect<rectangularCoordinates.ComponentProps>("RectangularCoordinates");

/** Reflective floor that mirrors the content above it. */
export const ReflectivePlane = defineEffect<reflectivePlane.ComponentProps>("ReflectivePlane");

/** Repeat the child content in grid, radial or linear layouts with per-instance variation. */
export const Repeater = defineEffect<repeater.ComponentProps>("Repeater");

/** Annular ring (donut) with adjustable radius and band thickness. */
export const Ring = defineEffect<ring.ComponentProps>("Ring");

/** Concentric animated ripples emanating from a point. */
export const Ripples = defineEffect<ripples.ComponentProps>("Ripples");

/** Wipe the content away in concentric rings pulsing out from a center point. */
export const RippleWipe = defineEffect<rippleWipe.ComponentProps>("RippleWipe");

/** Rounded rectangle with adjustable width, height, and corner rounding. */
export const RoundedRect = defineEffect<roundedRect.ComponentProps>("RoundedRect");

/** Adjust color saturation intensity. */
export const Saturation = defineEffect<saturation.ComponentProps>("Saturation");

/** Fine hairline scratches, like a worn film or scratched surface. */
export const Scratches = defineEffect<scratches.ComponentProps>("Scratches");

/** Adjust image sharpness using a convolution kernel. */
export const Sharpness = defineEffect<sharpness.ComponentProps>("Sharpness");

/** Broken glass effect with tectonic plate displacement. */
export const Shatter = defineEffect<shatter.ComponentProps>("Shatter");

/** Organic noise with animated movement. */
export const SimplexNoise = defineEffect<simplexNoise.ComponentProps>("SimplexNoise");

/** Animated wave with thickness and softness. */
export const SineWave = defineEffect<sineWave.ComponentProps>("SineWave");

/** Slice the content into strips that slide away in alternating directions. */
export const SliceWipe = defineEffect<sliceWipe.ComponentProps>("SliceWipe");

/** Realistic fluid smoke simulation with vorticity dynamics. */
export const Smoke = defineEffect<smoke.ComponentProps>("Smoke");

/** Fill a shape with swirling fluid smoke that interacts with the shape boundary. */
export const SmokeFill = defineEffect<smokeFill.ComponentProps>("SmokeFill");

/** Cursor-driven smoke that lingers, swirls, and dissipates with fluid dynamics. */
export const SmokeFlow = defineEffect<smokeFlow.ComponentProps>("SmokeFlow");

/** Inverts tones above a luminance threshold — a classic darkroom and photo effect. */
export const Solarize = defineEffect<solarize.ComponentProps>("Solarize");

/** Fill the canvas with a single solid color. */
export const SolidColor = defineEffect<solidColor.ComponentProps>("SolidColor");

/** Twinkling star glints over the bright parts of the layer inside. */
export const Sparkle = defineEffect<sparkle.ComponentProps>("Sparkle");

/** Map content onto a 3D sphere surface with depth distortion. */
export const Spherize = defineEffect<spherize.ComponentProps>("Spherize");

/** Rotating spiral pattern with animated movement. */
export const Spiral = defineEffect<spiral.ComponentProps>("Spiral");

/** Classic star polygon with straight sides and sharp pointed tips. */
export const Star = defineEffect<star.ComponentProps>("Star");

/** Applies a marbled stone relief and surface distortion to child content. */
export const Stone = defineEffect<stone.ComponentProps>("Stone");

/** Flowing ribbons of light with a multi-color gradient. */
export const Strands = defineEffect<strands.ComponentProps>("Strands");

/** Stretch content towards a direction from a center point. */
export const Stretch = defineEffect<stretch.ComponentProps>("Stretch");

/** Alternating colored stripes with animation. */
export const Stripes = defineEffect<stripes.ComponentProps>("Stripes");

/** Multi-light studio background with ambient motion. */
export const StudioBackground = defineEffect<studioBackground.ComponentProps>("StudioBackground");

/** Radial sunburst rays emanating from a center point. */
export const SunBurst = defineEffect<sunBurst.ComponentProps>("SunBurst");

/** Drapes child content over a 3D wave surface with perspective and lighting. */
export const Surface3D = defineEffect<surface3D.ComponentProps>("Surface3D");

/** Flowing swirl pattern with multi-layered noise. */
export const Swirl = defineEffect<swirl.ComponentProps>("Swirl");

/** Teardrop — a rounded bulb tapering to a sharp point. */
export const Teardrop = defineEffect<teardrop.ComponentProps>("Teardrop");

/** Text with any Google font — multi-line with wrapping, alignment and line height — rendered crisp via a glyph texture. */
export const Text = defineEffect<text.ComponentProps>("Text");

/** Iridescent thin-film edge. */
export const ThinFilm = defineEffect<thinFilm.ComponentProps>("ThinFilm");

/** Selective focus blur mimicking tilt-shift photography. */
export const TiltShift = defineEffect<tiltShift.ComponentProps>("TiltShift");

/** An Echo-style temporal trail — whatever moves sheds a smooth, decaying long-exposure ghost trail (frame-difference Motion mode works on opaque images and video; Alpha mode trails a shape's coverage), with optional zoom-feedback tunnels and rainbow light-painting. */
export const TimeTrail = defineEffect<timeTrail.ComponentProps>("TimeTrail");

/** Apply a color tint to the image. */
export const Tint = defineEffect<tint.ComponentProps>("Tint");

/** Trapezoid with adjustable top and bottom widths and height. */
export const Trapezoid = defineEffect<trapezoid.ComponentProps>("Trapezoid");

/** Tiling grid of equilateral triangles with optional animated row offsets. */
export const TriangularGrid = defineEffect<triangularGrid.ComponentProps>("TriangularGrid");

/** Map colors to three tones: shadows, midtones, highlights. */
export const Tritone = defineEffect<tritone.ComponentProps>("Tritone");

/** Quarter-circle arc tiles that connect to form organic, maze-like flowing curves. */
export const Truchet = defineEffect<truchet.ComponentProps>("Truchet");

/** Rotate and twist content around a center point. */
export const Twirl = defineEffect<twirl.ComponentProps>("Twirl");

/** Wipe the content away behind a set of parallel closing strips. */
export const VenetianBlinds = defineEffect<venetianBlinds.ComponentProps>("VenetianBlinds");

/** Vesica piscis (lens shape) formed by the intersection of two overlapping circles. */
export const Vesica = defineEffect<vesica.ComponentProps>("Vesica");

/** Analog VHS tape with intermittent tape damage, chroma bleed, and per-scanline noise. */
export const VHS = defineEffect<vHS.ComponentProps>("VHS");

/** Selective saturation adjustment protecting skin tones. */
export const Vibrance = defineEffect<vibrance.ComponentProps>("Vibrance");

/** Display a video with customizable playback and object-fit modes. */
export const VideoTexture = defineEffect<videoTexture.ComponentProps>("VideoTexture");

/** Darkens or tints the edges of the frame, drawing attention toward the center. */
export const Vignette = defineEffect<vignette.ComponentProps>("Vignette");

/** Cellular pattern where each pixel is colored by its distance to the nearest of many scattered points. */
export const Voronoi = defineEffect<voronoi.ComponentProps>("Voronoi");

/** Rebuild any shape out of voxels — a flat shape becomes chunky pixel art, a 3D shape a lit voxel model with smooth ambient occlusion, cast shadows, a key light you can orbit or drive, and per-cube color. */
export const Voxels = defineEffect<voxels.ComponentProps>("Voxels");

/** Translucent water — a refractive, light-absorbing body that deepens to its color with thickness, wrapped in a wind-driven wave surface that reflects a procedural sky, with Fresnel-bright grazing edges and foam breaking on the crests and shoreline. */
export const Water = defineEffect<water.ComponentProps>("Water");

/** Painterly watercolor look — Kuwahara flattening, pigment edge darkening, paper grain and bleeding. */
export const Watercolor = defineEffect<watercolor.ComponentProps>("Watercolor");

/** Wave-based distortion with multiple waveform types. */
export const WaveDistortion = defineEffect<waveDistortion.ComponentProps>("WaveDistortion");

/** Audio-visualizer waveform — equalizer bars, a filled wave, an oscilloscope line, or a dot matrix — driven by a simulated signal whose amplitude you can map to your own audio level. */
export const Waveform = defineEffect<waveform.ComponentProps>("Waveform");

/** Rotating banded wavelets that ripple as they animate. */
export const WaveletNoise = defineEffect<waveletNoise.ComponentProps>("WaveletNoise");

/** Interlaced textile weave pattern with two thread colors going over and under each other. */
export const Weave = defineEffect<weave.ComponentProps>("Weave");

/** Display a live webcam feed with customizable object-fit modes. */
export const WebcamTexture = defineEffect<webcamTexture.ComponentProps>("WebcamTexture");

/** Applies an interwoven fibrous fabric texture and distortion to child content. */
export const Wool = defineEffect<wool.ComponentProps>("Wool");

/** Cellular noise field — distance-based, with selectable feature combinations and fractal octaves. */
export const WorleyNoise = defineEffect<worleyNoise.ComponentProps>("WorleyNoise");

/** Radial zoom blur expanding from a center point. */
export const ZoomBlur = defineEffect<zoomBlur.ComponentProps>("ZoomBlur");
