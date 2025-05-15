// nocheckin

// Enum-like string unions (could be actual enums too)
enum TransitionType {
    Tween = "tween",
    Spring = "spring",
}

enum LayoutDirection {
    Vertical = "vertical",
    Horizontal = "horizontal",
}

enum StackAlignment {
    Start = "start",
    Center = "center",
    End = "end", // Assuming
    SpaceBetween = "space-between", // Assuming
}

enum StackDistribution {
    Start = "start",
    Center = "center",
    End = "end", // Assuming
    SpaceBetween = "space-between", // Assuming
}

// 0: Fixed, 1: Relative (%), 2: Fit-Content/Auto, 3: Fill (1fr)
enum DimensionType {
    Fixed = 0,
    Relative = 1,
    FitContent = 2,
    Fill = 3,
    AspectRatio = 4, // Height determined by aspect ratio
}

enum StyleAppearEffectTrigger {
    OnInView = "onInView",
    OnMount = "onMount",
    OnScrollDirection = "onScrollDirection",
}

enum StyleAppearEffectScrollDirection {
    Down = "down",
    Up = "up", // Assuming
    Both = "both", // Assuming
}

enum FloatingPlacement {
    Top = "top",
    Bottom = "bottom",
    Left = "left",
    Right = "right",
}

enum FloatingAlignment {
    Start = "start",
    Center = "center",
    End = "end",
}

enum GridDimensionType {
    Fixed = "fixed",
    Fit = "fit",
    MinMax = "minmax", // For columns
    Auto = "auto", // For rows
}

enum GridAlignment {
    Start = "start",
    Center = "center",
    End = "end",
    Stretch = "stretch", // Assuming
}

enum PathBooleanOperation {
    Union = 0, // Assuming
    Subtract = 4,
    Intersect = 2, // Assuming
    Exclude = 3, // Assuming
}

enum ExternalModuleType {
    Collection = "collection",
    CodeFile = "codeFile",
    CanvasComponent = "canvasComponent",
    SiteMetadata = "siteMetadata",
    Screen = "screen",
    Config = "config",
    CSS = "css",
    Prototype = "prototype",
    DraftCollection = "draftCollection",
}

enum CollectionVariableType {
    String = "string",
    Image = "image",
    Boolean = "boolean",
    Slug = "slug",
    Link = "link",
    RichText = "richtext",
    Number = "number",
    Color = "color",
    Date = "date", // Assuming
    File = "file", // Assuming
}

// Base interface for all nodes
interface BaseNode {
    __class: string;
    id: string;
    name: string | null;
    parentid: string | null;
    originalid?: string | null;
    duplicatedFrom?: string | string[] | null;
    visible?: boolean;
    children?: AnyNode[];
}

// Specific Node Interfaces
interface TransitionSettings {
    type: TransitionType | string;
    delay: number;
    duration: number;
    ease: number[];
    stiffness: number;
    damping: number;
    mass: number;
    durationBasedSpring?: boolean;
    bounce?: number;
}

interface EffectProperties {
    x: string;
    y: string;
    scale: number;
    opacity: number;
    rotate3d: boolean;
    rotate: number;
    rotateX: number;
    rotateY: number;
    transition: TransitionSettings;
    skewX?: number;
    skewY?: number;
    perspective?: number;
}

interface PageEffect {
    enter: EffectProperties;
    // exit?: EffectProperties; // Assuming
}

interface WebMetadata {
    title?: string;
    description?: string;
    language?: string;
    canonicalURL?: string; // "default" or URL
    reducedMotion?: boolean;
    optOutOfHTMLPlugin?: boolean;
    socialImage: string; // data:framer/asset-reference,...
    favicon?: string; // data:framer/asset-reference,...
    noIndexSite?: boolean;
}

interface CustomHTML {
    headStart: string;
    headEnd: string;
    bodyStart: string;
    bodyEnd: string;
}

interface RootNode extends BaseNode {
    __class: "RootNode";
    v: number;
    header: number[];
    version: number;
    hash: number;
    disableBackdropFilters: boolean;
    homePageNodeId: string;
    thumbnailNodeId: string;
    webMetadata: WebMetadata;
    customHTML: CustomHTML;
    globalPageEffect: PageEffect;
    publishPaths: Record<string, string>;
    publishRevisions: Record<string, number | null>;
    children: (WebPageNode | ExternalModulesListNode | ContentManagementNode | ColorStyleTokenListNode | LocalModulesListNode | CanvasPageNode | PresetsListNode | EntityRootNode | SmartComponentNode | RouteSegmentRootNode)[];
}

interface PreviewSettings {
    __class: "PreviewSettings";
    responsive: boolean;
    touch: boolean;
}

interface ActionControlValue {
    type: "string" | "enum" | "link" | "image" | "number" | "boolean" | "fusednumber" | "object" | "array" | "variableReference";
    value: string | number | boolean | LinkValue | ImageValue | Record<string, any> | any[];
}

interface ActionTrigger {
    identifier: string;
    actionIdentifier: string;
    controls: Record<string, ActionControlValue | { type: string; value: string | number | boolean}>; // Value can be complex
    meta?: Record<string, any>;
}

interface VariableReference {
    type: "variableReference";
    id: string;
    providerId: string;
}

interface ReplicaInfo {
    master: string;
    inheritsFrom?: string;
    overrides: Record<string, any>; // NodeID: { property: value, _deleted?: string[] }
}

interface BoxShadow {
    __class: "BoxShadow";
    id: string;
    type: "box" | "realistic" | string;
    color: string;
    x: number;
    y: number;
    inset: boolean;
    blur: number;
    spread: number;
    diffusion?: number;
    focus?: number;
}

interface FrameNode extends BaseNode {
    __class: "FrameNode";
    width: number | string; // Can be "1fr" etc.
    height: number | string;
    widthType?: DimensionType;
    heightType?: DimensionType;
    fillColor?: string;
    fillEnabled?: boolean;
    isMaster?: boolean;
    isVariant?: boolean;
    isBreakpoint?: boolean;
    variantTransition?: TransitionSettings;
    previewSettings?: PreviewSettings;
    overflow?: "visible" | "hidden" | "scroll";
    radius?: number;
    radiusTopLeft?: number;
    radiusTopRight?: number;
    radiusBottomRight?: number;
    radiusBottomLeft?: number;
    radiusPerCorner?: boolean;
    top?: number | string | null;
    left?: number | string | null;
    right?: number | string | null;
    bottom?: number | string | null;
    centerAnchorX?: number;
    centerAnchorY?: number;
    layout?: "stack" | "grid";
    gap?: number;
    stackAlignment?: StackAlignment | string;
    stackDirection?: LayoutDirection | string;
    stackDistribution?: StackDistribution | string;
    stackWrapEnabled?: boolean;
    padding?: number | string;
    paddingPerSide?: boolean;
    paddingTop?: number | string;
    paddingRight?: number | string;
    paddingBottom?: number | string;
    paddingLeft?: number | string;
    aspectRatio?: number | null;
    pageEffects?: Record<string, any>; // Define more if structure is known
    position?: "absolute" | "relative" | "fixed" | "sticky";
    positionStickyTop?: number;
    zIndex?: number;
    htmlTag?: string;
    ariaLabel?: string;
    onMouseEnter?: ActionTrigger[];
    onTap?: ActionTrigger[];
    borderEnabled?: boolean;
    borderWidth?: number;
    borderColor?: string;
    borderStyle?: string; // "solid"
    borderPerSide?: boolean;
    borderTop?: number;
    borderRight?: number;
    borderBottom?: number;
    borderLeft?: number;
    floatingPositionEnabled?: boolean;
    floatingPlacement?: FloatingPlacement | string;
    floatingAlignment?: FloatingAlignment | string;
    floatingOffsetX?: number;
    floatingOffsetY?: number;
    styleAppearEffectEnabled?: boolean;
    styleAppearEffectLocked?: boolean;
    styleAppearEffectTrigger?: StyleAppearEffectTrigger | string;
    styleAppearEffectScrollDirection?: StyleAppearEffectScrollDirection | string;
    styleAppearEffectThreshold?: number;
    styleAppearEffectAnimateOnce?: boolean;
    styleAppearEffectOpacity?: number;
    styleAppearEffectX?: number;
    styleAppearEffectY?: number;
    styleAppearEffectScale?: number;
    styleAppearEffectTransition?: TransitionSettings;
    styleAppearEffectRotate?: number;
    styleAppearEffectRotateX?: number;
    styleAppearEffectRotateY?: number;
    styleAppearEffectPerspective?: number;
    enterEffectEnabled?: boolean;
    enterEffectOpacity?: number;
    enterEffectX?: number;
    enterEffectY?: number;
    enterEffectScale?: number;
    enterEffectTransition?: TransitionSettings;
    enterEffectRotate3d?: boolean;
    enterEffectRotate?: number;
    enterEffectRotateX?: number;
    enterEffectRotateY?: number;
    enterEffectPerspective?: number;
    enterEffectSkewX?: number;
    enterEffectSkewY?: number;
    exitEffectEnabled?: boolean;
    exitEffectRotate3d?: boolean; // Assuming similar props for exit
    maxWidth?: string;
    opacity?: number;
    rotation?: number;
    whileHoverEnabled?: boolean;
    whileHoverOpacity?: number;
    whileHoverX?: number;
    whileHoverY?: number;
    whileHoverScale?: number;
    whileHoverRotate3d?: boolean;
    whileHoverRotate?: number;
    whileHoverRotateX?: number;
    whileHoverRotateY?: number;
    whileHoverSkewX?: number;
    whileHoverSkewY?: number;
    whileHoverTransition?: TransitionSettings;
    collectionFilters?: CollectionFilters;
    dataIdentifier?: string;
    collectionLimit?: number;
    collectionStartOffset?: number;
    scrollTargetEnabled?: boolean;
    styleTransformEffectEnabled?: boolean;
    styleTransformEffectViewportThreshold?: number;
    styleTransformEffectScrollTargets?: StyleTransformEffectTarget[];
    styleTransformEffectTrigger?: "onInView" | "onScroll";
    styleTransformEffectTransitionEnabled?: boolean;
    gridColumnCount?: number;
    gridRowCount?: number;
    gridAlignment?: GridAlignment | string;
    gridColumnWidthType?: GridDimensionType | string;
    gridColumnWidth?: number;
    gridColumnMinWidth?: number;
    gridRowHeightType?: GridDimensionType | string;
    gridRowHeight?: number | string;
    gridItemFillCellWidth?: boolean;
    gridItemFillCellHeight?: boolean;
    gridItemColumnSpan?: number;
    gridItemRowSpan?: number;
    fillType?: "color" | "image" | "gradient";
    fillImage?: string | VariableReference;
    fillImageOriginalName?: string | null;
    fillImagePixelWidth?: number | null;
    fillImagePixelHeight?: number | null;
    intrinsicWidth?: number | null;
    intrinsicHeight?: number | null;
    boxShadows?: BoxShadow[];
    altAttribute?: string;
    replicaInfo?: ReplicaInfo;
    viewportHeight?: number;
    title?: string; // For Smart Component variants
    gesture?: "hover" | "tap" | string; // For Smart Component variants
    link?: string | LinkValue; // For linking
    linkOpenInNewTab?: boolean;
    linkSmoothScroll?: boolean; // Assuming
    locked?: boolean;
    children?: AnyNode[];
    itemsOrder?: string[]; // For reordered children in grid/stack
    minWidth?: string | number; // From Social Links example
    maxHeight?: string | number; // From Footer example
}

interface StyleTransformEffectTarget {
    id: string;
    style: Partial<EffectProperties>; // Re-using, but might be simpler
}

interface RichTextNode extends BaseNode {
    __class: "RichTextNode";
    html: string;
    stylePresetHeading1?: string;
    stylePresetHeading2?: string;
    stylePresetHeading3?: string;
    stylePresetHeading4?: string;
    stylePresetHeading5?: string;
    stylePresetParagraph?: string;
    stylePresetLink?: string;
    linkTextColor?: string;
    linkTextDecoration?: string;
    userSelect?: "none" | string;
    textContent?: string | VariableReference;
    paragraphSpacing?: number;
    opacity?: number;
    textVerticalAlignment?: "top" | "center" | "bottom";
    // Text Effects
    textEffectBlur?: number;
    textEffectDelay?: number;
    textEffectEnabled?: boolean;
    textEffectOpacity?: number;
    textEffectReplay?: boolean;
    textEffectRotate?: number;
    textEffectRotate3d?: boolean;
    textEffectRotateX?: number;
    textEffectRotateY?: number;
    textEffectScale?: number;
    textEffectSkewX?: number;
    textEffectSkewY?: number;
    textEffectThreshold?: number;
    textEffectTokenization?: "line" | "word" | "character";
    textEffectTransition?: TransitionSettings;
    textEffectTrigger?: "onMount" | "onInView";
    textEffectType?: "appear" | string;
    textEffectX?: number;
    textEffectY?: number;
    // Style Appear Effects (from hero text)
    styleAppearEffectEnabled?: boolean;
    styleAppearEffectThreshold?: number;
    styleAppearEffectAnimateOnce?: boolean;
    styleAppearEffectOpacity?: number;
    styleAppearEffectX?: number;
    styleAppearEffectY?: number;
    styleAppearEffectScale?: number;
    styleAppearEffectTransition?: TransitionSettings;
    styleAppearEffectRotate?: number;
    styleAppearEffectRotateX?: number;
    styleAppearEffectRotateY?: number;
    styleAppearEffectPerspective?: number;
    styleAppearEffectLocked?: boolean;
    styleAppearEffectTrigger?: StyleAppearEffectTrigger | string;
    styleAppearEffectScrollDirection?: StyleAppearEffectScrollDirection | string;
    enterEffectEnabled?: boolean;
    enterEffectOpacity?: number;
    enterEffectX?: number;
    enterEffectY?: number;
    enterEffectScale?: number;
    enterEffectTransition?: TransitionSettings;
    enterEffectRotate3d?: boolean;
    enterEffectRotate?: number;
    enterEffectRotateX?: number;
    enterEffectRotateY?: number;
    enterEffectPerspective?: number;
    exitEffectEnabled?: boolean;
    exitEffectRotate3d?: boolean;
}

interface PathSegment {
    __class: "PathSegment";
    x: number;
    y: number;
    handleMirroring: "straight" | "symmetric" | "disconnected" | string;
    handleOutX?: number;
    handleOutY?: number;
    handleInX?: number;
    handleInY?: number;
    radius?: number;
}

interface PathNode extends BaseNode {
    __class: "PathNode";
    fillEnabled?: boolean;
    fillColor?: string;
    strokeEnabled?: boolean;
    strokeAlignment?: "center" | "inside" | "outside";
    strokeWidth?: number;
    strokeColor?: string;
    lineJoin?: "miter" | "round" | "bevel";
    lineCap?: "butt" | "round" | "square";
    strokeMiterLimit?: number;
    strokeDashArray?: string;
    strokeDashOffset?: number;
    pathSegments: PathSegment[];
    pathClosed?: boolean;
    width: number;
    height: number;
    x?: number;
    y?: number;
}

interface ShapeGroupNode extends BaseNode {
    __class: "ShapeGroupNode";
    width: number;
    height: number;
    x?: number;
    y?: number;
    children: (PathNode | BooleanShapeNode | ShapeGroupNode)[];
}

interface ShapeContainerNode extends BaseNode {
    __class: "ShapeContainerNode";
    contentHash: number;
    width: number;
    height: number;
    fillEnabled?: boolean;
    fillColor?: string;
    children: (PathNode | BooleanShapeNode | ShapeGroupNode)[];
    link?: LinkValue;
    title?: string;
    description?: string;
}

interface BooleanShapeNode extends BaseNode {
    __class: "BooleanShapeNode";
    fillEnabled?: boolean;
    fillColor?: string;
    pathBoolean: PathBooleanOperation | number;
    strokeEnabled?: boolean;
    strokeAlignment?: "center" | "inside" | "outside";
    strokeWidth?: number;
    strokeColor?: string;
    lineJoin?: "miter" | "round" | "bevel";
    lineCap?: "butt" | "round" | "square";
    strokeMiterLimit?: number;
    strokeDashArray?: string;
    strokeDashOffset?: number;
    width: number;
    height: number;
    x?: number;
    y?: number;
    children: PathNode[];
}

interface SVGNode extends BaseNode {
    __class: "SVGNode";
    intrinsicWidth: number;
    intrinsicHeight: number;
    svg: string;
    width: number | string;
    height: number | string;
}

interface LinkValue {
    type: "url" | "webPage";
    url?: string;
    webPageId?: string;
    pathVariables?: Record<string, VariableReference | string>;
}

interface ImageValue {
    type: "variableReference" | "asset"; // asset if not variable
    id?: string; // if variableReference
    providerId?: string; // if variableReference
    src?: string; // if asset (data:framer/...)
    alt?: string;
}

interface ComponentInstanceReference {
    type: "componentinstance";
    id: string;
    value: string; // Node ID
}

type ControlValueType = string | number | boolean | LinkValue | ImageValue | VariableReference | ComponentInstanceReference | Record<string, any> | any[];

interface ControlProperty {
    type: "string" | "number" | "boolean" | "enum" | "link" | "image" | "fusednumber" | "object" | "array" | string;
    value: ControlValueType;
    isFused?: boolean; // for fusednumber
}

interface CodeComponentNode extends BaseNode {
    __class: "CodeComponentNode";
    codeComponentIdentifier: string;
    [key: string]: any; // For $control__* properties and FrameNode properties
}

interface CollectionFilters {
    filters: {
        id: string;
        itemKey: string;
        transforms: any[];
    }[];
}

interface WebPageNode extends BaseNode {
    __class: "WebPageNode";
    baseVariantId: string;
    webMetadata: WebMetadata;
    customHTML: CustomHTML;
    moduleSourceRevisionHint?: number;
    moduleSourceRevision?: number;
    moduleSourceRevisionCommittedHint?: number;
    pagePath?: string;
    dataIdentifier?: string; // For CMS pages
    children: (FrameNode | SmartComponentNode)[];
}

interface ExternalModulesListNode extends BaseNode {
    __class: "ExternalModulesListNode";
    children: ExternalModuleNode[];
}

interface ExternalModuleSaveAnnotation {
    framerContractVersion: number | string;
    framerIntrinsicHeight?: string | number;
    framerIntrinsicWidth?: string | number;
    framerCanvasComponentVariantDetails?: string;
    framerImmutableVariables?: "true" | boolean;
    framerAcceptsLayoutTemplate?: boolean;
    framerAutoSizeImages?: boolean;
    framerDisplayContentsDiv?: boolean;
    framerScrollSections?: string;
    framerColorSyntax?: boolean;
    framerComponentViewportWidth?: boolean;
    framerSupportedLayoutHeight?: "any" | "fixed";
    framerSupportedLayoutWidth?: "any" | "fixed";
    framerDisableUnlink?: "*";
    framerVariables?: string;
    framerData?: string;
    framerSlug?: string;
    framerRecordIdKey?: string;
    metadataVersion?: { framerContractVersion: number | string };
}

interface ExternalModuleGroup {
    type: "team" | string;
    id: string;
    name: string;
    imageURL: string | null;
}

interface ExternalModuleNode extends BaseNode {
    __class: "ExternalModuleNode";
    title: string;
    annotations: ExternalModuleSaveAnnotation;
    codeComponentIdentifier?: string;
    ownerType: "project" | string;
    ownerId: string;
    type: ExternalModuleType | string;
    group: ExternalModuleGroup;
    updateSaveId: string;
    intrinsicWidth?: number;
    intrinsicHeight?: number;
    scopeNodeId?: string;
    namespaceId?: string;
}

interface ContentManagementNode extends BaseNode {
    __class: "ContentManagementNode";
    children: CollectionNode[];
}

interface VariableOptions {
    maxLength?: number;
    placeholder?: string;
    displayTextArea?: boolean;
}

interface VariableDefinition {
    id: string;
    exposeInProps: boolean;
    type: CollectionVariableType | string;
    initialValue: string | boolean | number | null;
    name: string;
    required?: boolean;
    options?: VariableOptions;
    associatedStringVariable?: string;
    fallbackValue?: "initialValue" | "associatedVariable";
}

interface CollectionNode extends BaseNode {
    __class: "CollectionNode";
    variables: VariableDefinition[];
    children: CollectionItemNode[];
}

interface CollectionItemControlValue {
    type: CollectionVariableType | string;
    value: string | boolean | number | LinkValue | ImageValue; // Allow basic types too
    alt?: string;
}

interface CollectionItemNode extends BaseNode {
    __class: "CollectionItemNode";
    [key: string]: CollectionItemControlValue | any; // For $control__* (variable values)
}

interface ColorStyleTokenListNode extends BaseNode {
    __class: "ColorStyleTokenListNode";
    children: ColorStyleTokenNode[];
}

interface ColorStyleTokenNode extends BaseNode {
    __class: "ColorStyleTokenNode";
    light: string;
    dark?: string;
}

interface LocalModuleSave {
    treeVersion: number;
    moduleId: string;
    saveId: string;
    imports: string[];
    title: string;
    name: string;
    type: ExternalModuleType | string;
    sourceRevision?: number;
    annotations?: Record<string, ExternalModuleSaveAnnotation | any>;
}

interface LocalModuleNode extends BaseNode {
    __class: "LocalModuleNode";
    save: LocalModuleSave;
}

interface LocalModulesListNode extends BaseNode {
    __class: "LocalModulesListNode";
    children: LocalModuleNode[];
}

interface CanvasPageNode extends BaseNode {
    __class: "CanvasPageNode";
    homeNodeId: string;
    children: FrameNode[];
}


interface TextStylePresetNode extends BaseNode {
    __class: "TextStylePresetNode";
    paragraphSpacing?: number;
    tag?: string;
    font?: string;
    textColor?: string;
    fontSize?: number;
    textAlignment?: "left" | "center" | "right" | "justify";
    fontBold?: string;
    isMaster?: boolean;
    lineHeight?: [number, "em" | "px" | "%" | ""];
    breakpointWidth?: number;
    textStrokeWidth?: string | number;
    textStrokeColor?: string;
}

interface PresetsListNode extends BaseNode {
    __class: "PresetsListNode";
    children: TextStylePresetNode[];
}

interface EntityTypeRootNode extends BaseNode {} // Base for entity roots

interface BlockquoteEntityTypeRootNode extends EntityTypeRootNode { __class: "BlockquoteEntityTypeRootNode"; }
interface CMSEntityTypeRootNode extends EntityTypeRootNode { __class: "CMSEntityTypeRootNode"; }
interface CodeFileEntityTypeRootNode extends EntityTypeRootNode { __class: "CodeFileEntityTypeRootNode"; }
interface ColorEntityTypeRootNode extends EntityTypeRootNode { __class: "ColorEntityTypeRootNode"; }
interface ComponentEntityTypeRootNode extends EntityTypeRootNode { __class: "ComponentEntityTypeRootNode"; }
interface InlineCodeEntityTypeRootNode extends EntityTypeRootNode { __class: "InlineCodeEntityTypeRootNode"; }
interface LinkEntityTypeRootNode extends EntityTypeRootNode { __class: "LinkEntityTypeRootNode"; }
interface TextEntityTypeRootNode extends EntityTypeRootNode { __class: "TextEntityTypeRootNode"; }

interface EntityRootNode extends BaseNode {
    __class: "EntityRootNode";
    children: (
        | BlockquoteEntityTypeRootNode
        | CMSEntityTypeRootNode
        | CodeFileEntityTypeRootNode
        | ColorEntityTypeRootNode
        | ComponentEntityTypeRootNode
        | InlineCodeEntityTypeRootNode
        | LinkEntityTypeRootNode
        | TextEntityTypeRootNode
    )[];
}

interface SmartComponentVariable extends VariableDefinition {}

interface SmartComponentNode extends BaseNode {
    __class: "SmartComponentNode";
    baseVariantId: string;
    variables?: SmartComponentVariable[];
    children: FrameNode[]; // Variants
    moduleSourceRevisionHint?: number;
    moduleSourceRevision?: number;
    moduleSourceRevisionCommittedHint?: number;
}

interface GradientColorStop {
    __class: "GradientColorStop";
    value: string;
    position: number;
    id: string;
}

interface LinearGradient {
    __class: "LinearGradient";
    alpha: number;
    angle: number;
    stops: GradientColorStop[];
}

interface RadialGradient {
    __class: "RadialGradient";
    alpha: number;
    widthFactor: number;
    heightFactor: number;
    centerAnchorX: number;
    centerAnchorY: number;
    stops: GradientColorStop[];
}

interface ConicGradient {
    __class: "ConicGradient";
    alpha: number;
    angle: number;
    centerAnchorX: number;
    centerAnchorY: number;
    stops: GradientColorStop[];
}

interface RouteSegmentNode extends BaseNode {
    __class: "RouteSegmentNode";
    segment: string;
    webPageId: string;
    dataIdentifier?: string;
    children: RouteSegmentNode[];
}

interface RouteSegmentRootNode extends BaseNode {
    __class: "RouteSegmentRootNode";
    children: RouteSegmentNode[];
}


// Union type for all possible node classes
type AnyNode =
    | RootNode
    | WebPageNode
    | FrameNode
    | PreviewSettings
    | RichTextNode
    | CodeComponentNode
    | PathNode
    | ShapeContainerNode
    | ShapeGroupNode
    | BooleanShapeNode
    | SVGNode
    | PathSegment
    | ExternalModulesListNode
    | ExternalModuleNode
    | ContentManagementNode
    | CollectionNode
    | CollectionItemNode
    | ColorStyleTokenListNode
    | ColorStyleTokenNode
    | LocalModulesListNode
    | LocalModuleNode
    | CanvasPageNode
    | PresetsListNode
    | TextStylePresetNode
    | EntityRootNode
    | BlockquoteEntityTypeRootNode
    | CMSEntityTypeRootNode
    | CodeFileEntityTypeRootNode
    | ColorEntityTypeRootNode
    | ComponentEntityTypeRootNode
    | InlineCodeEntityTypeRootNode
    | LinkEntityTypeRootNode
    | TextEntityTypeRootNode
    | SmartComponentNode
    | GradientColorStop
    | LinearGradient
    | RadialGradient
    | ConicGradient
    | BoxShadow
    | RouteSegmentNode
    | RouteSegmentRootNode;

// Top-level document structure
interface FramerDocument {
    v: number;
    header: number[];
    version: number;
    hash: number;
    root: RootNode;
}

