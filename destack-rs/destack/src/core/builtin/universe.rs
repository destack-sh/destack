//! destack.core.builtin.universe

#![destack::partial(destack.core.builtin.universe, file)]

#[destack::generated(ModuleType, -, block)]
/// Built-in module types.
pub enum ModuleType {
    /// Root module for the entire Universe
    Root = 1,
    /// Module for an entire UniverseDomain
    Domain = 2,
    /// Module for an entire UniverseCategory
    Category = 3,
    /// Module for one or more Objects
    Object = 4,
}

#[destack::generated(EnumType, -, block)]
/// EnumType
pub enum EnumType {
    ModuleType = 2,
    EnumType = 3,
    NodeType = 4,
    StructType = 5,
    TraitType = 6,
    HandleType = 7,
    UniverseDomain = 10,
    UniverseCategory = 11,
    Materialization = 20,
    ProcessFlag = 21,
    ExtensionFlag = 22,
    RuntimePlatform = 30,
    RuntimeLanguage = 31,
    RuntimeType = 32,
    EventStatus = 40,
    StringCasing = 50,
    PrimitiveType = 200000,
    TypeCardinality = 200001,
    ScalarType = 200002,
    ValueFactory = 200003,
    PropertyZone = 200006,
    ReferenceType = 200010,
    TextSpanType = 200100,
    TextStyleFlag = 200101,
    IconType = 200110,
    ChangeType = 200200,
    EditOperationType = 200201,
    ConditionalType = 200300,
    AggregationType = 200301,
    SortMode = 200302,
    SortType = 200303,
    JoinType = 200304,
    ExpressionType = 200306,
    QueryType = 200320,
    Encoding = 1000000,
    EncoderFlag = 1000001,
    EncoderStability = 1000002,
    ClientType = 3000200,
    BranchType = 3100300,
    SnapshotType = 3100400,
    SnapshotStatus = 3100401,
    FolderType = 3100500,
    IndexType = 10000100,
    ConstraintType = 10000200,
    MigrationType = 10000300,
    RunStatus = 10101000,
    FunctionOperator = 10100400,
    MethodType = 10100500,
    ActionType = 10100600,
    TriggerType = 10100700,
    TimerType = 10100800,
    DayOfWeek = 10100801,
    Month = 10100802,
    ScheduleFrequency = 10100803,
    LogLevel = 10101100,
    RoleType = 10300300,
    SanctionType = 10300400,
    EntitlementType = 10300500,
    NotificationStatus = 11001301,
    RigidMotionMode = 20200100,
    JointFlag = 20201000,
    MouseButton = 20300000,
    TransitionType = 30500000,
    SpringType = 30500001,
    EffectType = 30500100,
    RepeatType = 30500002,
    Easing = 30500003,
    TextSplitType = 30500004,
    OffscreenBehavior = 30500005,
    ColorType = 31000000,
    ColorShade = 31000001,
    ColorHue = 31000002,
    ColorIntent = 31000003,
    FillType = 31000400,
    FillPosition = 31000401,
    FillSize = 31000402,
    FontType = 31000500,
    FontWeight = 31000501,
    FontSize = 31000502,
    TextAlign = 31000503,
    TextDecoration = 31000504,
    TextTransform = 31000505,
    BorderType = 31000600,
    ShadowType = 31000700,
    ShadowPosition = 31000701,
    GradientType = 31000800,
    StrokeType = 31000900,
    LengthType = 40100001,
    Layout = 40100002,
    Distribute = 40100003,
    Align = 40100004,
    Direction = 40100005,
    Overflow = 40100006,
    Anchor = 40100007,
    Region = 50000001,
    RegionArea = 50000002,
    RegionContinent = 50000003,
    MachineType = 50000000,
}

#[destack::generated(NodeType, -, block)]
/// NodeType
pub enum NodeType {
    /// Root of all Nodes
    Node = 1,
    /// Versioned, stateful Node
    Entity = 2,
    /// Immutable datum of something happening
    Event = 3,
    ChangeEvent = 201000,
    /// The Destack computational universe
    Universe = 3000000,
    User = 3000100,
    Client = 3000200,
    Organization = 3000300,
    /// Team in an Organization
    Team = 3000400,
    /// Universal Space
    Space = 3100000,
    Branch = 3100300,
    /// Point in Space-time
    Snapshot = 3100400,
    /// Sub-space of a Space
    Folder = 3100500,
    /// Custom Event Definition
    CustomEventDefinition = 10000000,
    /// Custom Struct Definition
    CustomStructDefinition = 10000100,
    /// Custom Error Definition
    CustomErrorDefinition = 10000300,
    /// Custom Message Definition
    CustomMessageDefinition = 10000200,
    /// Custom Enum Definition
    CustomEnumDefinition = 10000500,
    /// Custom Property Definition
    CustomPropertyDefinition = 10001000,
    /// Custom Option Definition
    CustomOptionDefinition = 10001100,
    Tag = 10010000,
    /// Index of an Entity
    Index = 10010100,
    /// Constraint of an Entity
    Constraint = 10010200,
    /// Migration of an Entity
    Migration = 10011000,
    /// Migration Operation of an Entity
    MigrationOperation = 10011100,
    File = 10020000,
    Environment = 10100100,
    Script = 10100200,
    /// Custom Event instance
    CustomEvent = 10100300,
    Function = 10101000,
    Method = 10101100,
    Action = 10101200,
    Trigger = 10102000,
    TriggerEvent = 10102001,
    Timer = 10102100,
    TimerEvent = 10102101,
    TimerStartedEvent = 10102102,
    TimerPausedEvent = 10102103,
    TimerResumedEvent = 10102104,
    TimerCompletedEvent = 10102105,
    TimerCancelledEvent = 10102106,
    Run = 10110000,
    RunEvent = 10110001,
    RunStartedEvent = 10110002,
    RunPauseRequestedEvent = 10110003,
    RunPausedEvent = 10110004,
    RunResumeRequestedEvent = 10110005,
    RunResumedEvent = 10110006,
    RunStopRequestedEvent = 10110007,
    RunFailedEvent = 10110008,
    RunCompletedEvent = 10110009,
    SpanEvent = 10110010,
    LogEvent = 10110011,
    /// Permission for something
    Permission = 10300000,
    /// Membership to something
    Membership = 10300100,
    MembershipEvent = 10300101,
    MembershipJoinedEvent = 10300102,
    MembershipLeftEvent = 10300103,
    /// Invite to a Space/Folder
    Invite = 10300200,
    InviteEvent = 10300201,
    InviteSentEvent = 10300202,
    InviteRescindedEvent = 10300203,
    InviteAcceptedEvent = 10300204,
    InviteRejectedEvent = 10300205,
    /// Role in something
    Role = 10300300,
    RoleEvent = 10300301,
    RoleAssignedEvent = 10300302,
    RoleUnassignedEvent = 10300303,
    /// Temporary or permanent restriction
    Sanction = 10300400,
    SanctionEvent = 10300401,
    SanctionRequestedEvent = 10300402,
    SanctionGrantedEvent = 10300403,
    SanctionRevokedEvent = 10300404,
    SanctionExpiredEvent = 10300405,
    /// Temporary or permanent grant
    Entitlement = 10300500,
    EntitlementEvent = 10300501,
    EntitlementRequestedEvent = 10300502,
    EntitlementGrantedEvent = 10300503,
    EntitlementRevokedEvent = 10300504,
    EntitlementExpiredEvent = 10300505,
    Star = 11001100,
    StarEvent = 11001101,
    StarAddedEvent = 11001102,
    StarRemovedEvent = 11001103,
    Reaction = 11001200,
    ReactionEvent = 11001201,
    ReactionAddedEvent = 11001202,
    ReactionRemovedEvent = 11001203,
    Follow = 11001300,
    FollowEvent = 11001301,
    FollowAddedEvent = 11001302,
    FollowRemovedEvent = 11001303,
    Notification = 11001400,
    NotificationEvent = 11001401,
    NotificationSentEvent = 11001402,
    NotificationRescindedEvent = 11001403,
    NotificationReadEvent = 11001404,
    NotificationDismissedEvent = 11001405,
    NotificationExpiredEvent = 11001406,
    Entity2D = 20000000,
    Shape2D = 20000100,
    PointShape2D = 20001000,
    CircleShape2D = 20001100,
    CapsuleShape2D = 20001200,
    RectangleShape2D = 20001300,
    SegmentShape2D = 20001400,
    PathShape2D = 20001500,
    EllipseShape2D = 20001600,
    HalfspaceShape2D = 20001700,
    ConvexShape2D = 20001800,
    MeshShape2D = 20001900,
    Entity3D = 20010000,
    Shape3D = 20010100,
    PointShape3D = 20011000,
    SphereShape3D = 20011100,
    CapsuleShape3D = 20011200,
    BoxShape3D = 20011300,
    SegmentShape3D = 20011400,
    PolylineShape3D = 20011500,
    EllipsoidShape3D = 20011600,
    CylinderShape3D = 20011700,
    ConeShape3D = 20011800,
    PlaneShape3D = 20011900,
    ConvexShape3D = 20012000,
    MeshShape3D = 20012100,
    Body2D = 20200000,
    BodyEvent = 20200001,
    BodySleepEvent = 20200002,
    BodyWakeEvent = 20200003,
    RigidBody2D = 20200100,
    SoftBody2D = 20200200,
    Joint2D = 20201000,
    JointEvent = 20201001,
    JointBreakEvent = 20201002,
    HingeJoint2D = 20201100,
    PrismaticJoint2D = 20201200,
    FixedJoint2D = 20201300,
    RopeJoint2D = 20201400,
    WheelJoint2D = 20201500,
    Body3D = 20210000,
    RigidBody3D = 20210100,
    SoftBody3D = 20210200,
    Joint3D = 20211000,
    HingeJoint3D = 20211100,
    PrismaticJoint3D = 20211200,
    FixedJoint3D = 20211300,
    RopeJoint3D = 20211400,
    WheelJoint3D = 20211500,
    SphericalJoint3D = 20211600,
    Collider2D = 20220000,
    ColliderEvent = 20220001,
    ColliderContactEvent = 20220002,
    Collider3D = 20220100,
    InputEvent = 20300000,
    Pointer2D = 20300100,
    PointerEvent = 20300101,
    PointerDownEvent = 20300102,
    PointerUpEvent = 20300103,
    PointerMoveEvent = 20300104,
    PointerEnterEvent = 20300105,
    PointerOverEvent = 20300106,
    PointerLeaveEvent = 20300107,
    PointerLongPressEvent = 20300108,
    Mouse2D = 20301000,
    MouseEvent = 20301101,
    ClickEvent = 20301102,
    SingleClickEvent = 20301103,
    DoubleClickEvent = 20301104,
    TripleClickEvent = 20301105,
    WheelEvent = 20301106,
    KeyEvent = 20302001,
    KeyDownEvent = 20302002,
    KeyUpEvent = 20302003,
    KeyPressEvent = 20302004,
    DragEvent = 20303001,
    DragStartEvent = 20303002,
    DragEndEvent = 20303003,
    DragOverEvent = 20303004,
    DragEnterEvent = 20303005,
    DragLeaveEvent = 20303006,
    DropEvent = 20303007,
    ClipboardEvent = 20304001,
    CopyEvent = 20304002,
    CutEvent = 20304003,
    PasteEvent = 20304004,
    FocusEvent = 20305001,
    FocusInEvent = 20305002,
    FocusOutEvent = 20305003,
    Audio = 30000000,
    Image = 30100000,
    Video = 30200000,
    Model = 30300000,
    TransitionTemplate = 30500000,
    EffectTemplate = 30500100,
    Theme = 31000000,
    Palette = 31000100,
    Style = 31000200,
    ColorStyle = 31001000,
    FillStyle = 31001100,
    FontStyle = 31001200,
    BorderStyle = 31001300,
    ShadowStyle = 31001400,
    GradientStyle = 31001500,
    StrokeStyle = 31001600,
    Document = 32000000,
    Stage = 40000000,
    /// Scene of an Application
    Scene = 40000100,
    SceneEvent = 40000101,
    /// Layer of a Scene
    Layer = 40000200,
    /// View in a Scene
    View2D = 40100000,
    LayoutView2D = 40100100,
    /// Fixed Container
    FrameView2D = 40100200,
    /// Label Container
    LabelView2D = 40100300,
    /// Split Container
    SplitView2D = 40100400,
    ContentView2D = 40100500,
    /// Text
    TextView2D = 40100501,
    InputView2D = 40100600,
    /// Number Input
    NumberInputView2D = 40100601,
    /// Slider Input
    SliderInputView2D = 40100602,
    /// Machine for ephemeral computing
    Machine = 50000000,
    Metric = 50100000,
    MeasurementEvent = 50100001,
    GaugeMetric = 50100100,
    GaugeMeasurementEvent = 50100101,
    CounterMetric = 50100200,
    CounterMeasurementEvent = 50100201,
    HistogramMetric = 50100300,
    HistogramMeasurementEvent = 50100301,
    Locale = 60000000,
}

#[destack::generated(StructType, -, block)]
/// StructType
pub enum StructType {
    /// Root of all Structs
    Struct = 1,
    /// Error
    Error = 2,
    /// Message
    Message = 3,
    Definition = 100000,
    SchemaDefinition = 100001,
    ModuleDefinition = 100002,
    ObjectDefinition = 100003,
    NodeDefinition = 100004,
    StructDefinition = 100005,
    HandleDefinition = 100006,
    EnumDefinition = 100007,
    PropertyDefinition = 100100,
    ConstantDefinition = 100101,
    OptionDefinition = 100102,
    Type = 200000,
    NumberConstraint = 200001,
    StringConstraint = 200002,
    CollectionConstraint = 200003,
    Value = 200050,
    NamedValue = 200051,
    Expression = 200100,
    Join = 200101,
    Aggregation = 200102,
    Condition = 200103,
    Sort = 200104,
    Select = 200105,
    Query = 200200,
    NodeIdentityReference = 200300,
    NodeSpatialReference = 200301,
    NodeTemporalReference = 200302,
    PropertyReference = 200360,
    Text = 200400,
    TextSpan = 200401,
    Icon = 200500,
    EditOperation = 201100,
    UniverseSignupRequest = 3000000,
    UniverseSignupResponse = 3000001,
    UniverseSpawnRequest = 3000002,
    UniverseSpawnResponse = 3000003,
    /// Custom Struct Instance
    CustomStruct = 10000100,
    /// Custom Error Instance
    CustomError = 10000200,
    /// Custom Message Instance
    CustomMessage = 10000300,
    TagDefinition = 10010000,
    IndexDefinition = 10010100,
    ConstraintDefinition = 10010200,
    MigrationDefinition = 10011000,
    MigrationOperationDefinition = 10011100,
    Schedule = 10100810,
    FunctionDefinition = 10101000,
    MethodDefinition = 10101100,
    ActionDefinition = 10101200,
    PermissionDefinition = 10300000,
    Vector2 = 20000010,
    Vector2I = 20000011,
    Vector3 = 20000012,
    Vector3I = 20000013,
    Vector4 = 20000014,
    Vector4I = 20000015,
    Quaternion = 20000020,
    Mesh2 = 20000030,
    Mesh3 = 20000031,
    Form2D = 20000100,
    Point2D = 20001000,
    Circle2D = 20001100,
    Capsule2D = 20001200,
    Rectangle2D = 20001300,
    Segment2D = 20001400,
    Path2D = 20001500,
    Ellipse2D = 20001600,
    Halfspace2D = 20001700,
    Form3D = 20010300,
    Point3D = 20011000,
    Sphere3D = 20011100,
    Capsule3D = 20011200,
    Box3D = 20011300,
    Segment3D = 20011400,
    Polyline3D = 20011500,
    Ellipsoid3D = 20011600,
    Cylinder3D = 20011700,
    Cone3D = 20011800,
    Plane3D = 20011900,
    JointSpring = 20201001,
    JointScalarLimit = 20201002,
    JointConeLimit = 20201003,
    JointTwistLimit = 20201004,
    JointBreakLimit = 20201005,
    JointMotor = 20201006,
    JointFrame2D = 20201007,
    JointFrame3D = 20201008,
    Transition = 30500000,
    Effect = 30500100,
    Color = 31001000,
    Fill = 31001100,
    Font = 31001200,
    Border = 31001300,
    Shadow = 31001400,
    Gradient = 31001500,
    GradientStop = 31001501,
    Stroke = 31001600,
    StrokeCap = 31001601,
    StrokePath = 31001602,
    StrokePoint = 31001603,
    Length = 40100000,
    Offset2 = 40100001,
    Grid2 = 40100002,
    GridSpan2 = 40100003,
    Inset2 = 40100004,
    Corner2 = 40100005,
    Axis2 = 40100006,
    Axis3 = 40100007,
}

#[destack::generated(TraitType, -, block)]
/// TraitType
pub enum TraitType {
    /// Is ordered
    Ordered = 1,
    /// Is a Resource
    Resource = 2,
    /// Is a Variant
    Variant = 3,
    /// Is a Template
    Template = 4,
    /// Can be run
    Runnable = 10100000,
    /// Is ownable
    Ownable = 10300000,
    /// Is owned
    Owned = 10300001,
    /// Is joinable
    Joinable = 10300002,
    /// Is an Actor
    Actor = 10300003,
    /// Can be starred
    Starable = 11000100,
    /// Can be reacted to
    Reactable = 11000200,
    /// Can be followed
    Followable = 11000300,
    /// Can be interacted with
    Interactive = 20300000,
    /// Can be dragged
    Draggable = 20300001,
    /// Can be selected
    Selectable = 20300002,
}

#[destack::generated(HandleType, -, block)]
/// The type of a Handle.
pub enum HandleType {
    Handle = 1,
    Graph = 1100000,
    Session = 2100000,
    Context = 2100100,
    Logger = 2100200,
    Tracer = 2100300,
}

#[destack::generated(UniverseDomain, -, block)]
/// The Destack Computational Universe is organized into domains.
pub enum UniverseDomain {
    /// Intrinsics: Basic atoms the rest of the Universe is built on.
    Core = 1,
    /// Scaffolding: Common Universe blocks that span domains.
    Basics = 10000000,
    /// Modeling: The Universe as a simulation.
    Simulation = 20000000,
    /// Imagining: The Universe as a canvas.
    Imagination = 30000000,
    /// Presenting: The Universe on stage.
    Presentation = 40000000,
    /// Operating: Building the Universe.
    Production = 50000000,
    /// Distributing: Integrating the Universe.
    Distribution = 60000000,
}

#[destack::generated(UniverseCategory, -, block)]
/// How the Destack Computational Universe is organized (domains > categories).
pub enum UniverseCategory {
    /// Primitives and intrinsics
    Builtin = 1,
    /// Builtin definitions
    Definition = 100000,
    /// Shared definitions
    Common = 200000,
    /// Serialization and packing
    Encoding = 1000000,
    /// Storage and synchronization
    Persistence = 1100000,
    /// Code generation and compilation
    Generation = 2000000,
    /// Local runtime integration
    Local = 2100000,
    /// Global computational universe
    Universe = 3000000,
    /// Spacetime organization
    Space = 3100000,
    /// Entity management
    Entity = 10000000,
    /// Logic, scripting and behavior
    Script = 10100000,
    /// Artificial intelligence
    Intelligence = 10200000,
    /// Identity, authentication and authorization
    Access = 10300000,
    /// Quality assurance
    Quality = 10400000,
    /// Interactions, reputation and trust
    Social = 11000000,
    /// Accounting and finance
    Finance = 11100000,
    /// Editing the Universe
    Editor = 19000000,
    /// Geometric representations
    Geometry = 20000000,
    /// Geographic representations
    Geography = 20100000,
    /// Physics simulation
    Physics = 20200000,
    /// Sensing and interaction
    Perception = 20300000,
    /// Audio and sound production
    Audio = 30000000,
    /// Image and photo production
    Image = 30100000,
    /// Video production
    Video = 30200000,
    /// Modeling, sculpting, CSG, CAD
    Model = 30300000,
    /// Drawing and painting
    Paint = 30400000,
    /// Motion choreography
    Animation = 30500000,
    /// Appearance and theming
    Style = 31000000,
    /// Document processing
    Document = 32000000,
    /// Staging and viewing
    Scene = 40000000,
    /// View construction
    View = 40100000,
    /// Render pipelines, post-processing
    Rendering = 40200000,
    /// Camera and viewport
    Camera = 40300000,
    /// Shader programming
    Shaders = 40400000,
    /// Material rendering
    Material = 40500000,
    /// Light and shadows
    Lighting = 40600000,
    /// Cloud computing infrastructure
    Cloud = 50000000,
    /// Telemetry on everything
    Observability = 50100000,
    /// User experience
    Experience = 50200000,
    /// Localization and internationalization
    Localization = 60000000,
    /// Legal, compliance and policy
    Legal = 60100000,
    /// Billing and monetization
    Commerce = 60200000,
}
