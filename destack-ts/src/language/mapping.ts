import type {
  Entitlement,
  EntitlementEvent,
  EntitlementEventType,
  EntitlementType,
  Invite,
  InviteEvent,
  InviteEventType,
  Membership,
  MembershipEvent,
  MembershipEventType,
  MembershipPermission,
  Permission,
  PermissionType,
  Role,
  RoleEvent,
  RoleEventType,
  Sanction,
  SanctionEvent,
  SanctionEventType,
  SanctionType,
} from "@destack/language/access";
import type {
  AnnotationShape,
  ArrowHeadType,
  ArrowShape,
  Canvas,
  CanvasType,
  IsShape,
  LineShape,
  LineType,
  PlaneShape,
  PlaneShapeType,
} from "@destack/language/canvas";
import type {
  Aggregation,
  AggregationType,
  Align,
  Analytic,
  Axis2,
  Axis3,
  Branch,
  CascadeAction,
  Change,
  ChangeDebounce,
  ChangeResult,
  ChangeStatus,
  ClientType,
  Cloud,
  CollectionConstraint,
  Condition,
  ConditionalType,
  ConstantDefinition,
  Corners,
  CounterMeasurement,
  CounterMetric,
  CustomEntity,
  CustomEntityDefinition,
  CustomEnumDefinition,
  CustomEvent,
  CustomEventDefinition,
  CustomOption,
  CustomProperty,
  CustomPropertyType,
  CustomStructDefinition,
  DefaultFactory,
  Dimension,
  DimensionType,
  Direction,
  Distribute,
  EdgeDirection,
  EdgeType,
  Edit,
  EditEvent,
  EditOperation,
  EditType,
  Entity,
  EnumDefinition,
  EnumType,
  EnvironmentType,
  Event,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  GaugeMeasurement,
  GaugeMetric,
  Global,
  Grid,
  GridSpan,
  HasIcon,
  HasName,
  HasSlug,
  Histogram,
  HistogramMeasurement,
  HistogramMetric,
  Icon,
  IconType,
  Indexed,
  Insets,
  IsActionable,
  IsArchivable,
  IsCustomNode,
  IsCustomNodeDefinition,
  IsDeletable,
  IsExtensible,
  IsFollowable,
  IsFrozen,
  IsJoinable,
  IsOrdered,
  IsOwnable,
  IsOwner,
  IsReactable,
  IsRunnable,
  IsScriptable,
  IsSettings,
  IsSourceable,
  IsStarable,
  IsSubject,
  IsTaggable,
  IsTracked,
  IsVisual,
  Join,
  JoinType,
  JoinablePermission,
  Layout,
  Length,
  LengthUnit,
  LikeFollow,
  LikeInvite,
  LikeMembership,
  LikeTag,
  MaterializationType,
  Measurement,
  Metric,
  ModeType,
  NodeConstraint,
  NodeDefinition,
  NodePermission,
  NodeReference,
  NodeType,
  NumberConstraint,
  NumberFormat,
  ObjectReference,
  ObjectType,
  OperatingSystem,
  OptionDefinition,
  Overflow,
  Particle,
  PermissionDefinition,
  PlatformType,
  Position,
  PositionType,
  PrimitiveType,
  PropertyDefinition,
  PropertyReference,
  PropertyReferenceType,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  QueryUpdate,
  QueryUpdateType,
  Region,
  RegionArea,
  RegionContinent,
  RelationReference,
  RelationType,
  Resource,
  ResourceStatus,
  RoleType,
  RuntimeType,
  ScalarType,
  Scope,
  Select,
  Selection,
  Snapshot,
  Sort,
  SortMode,
  SortType,
  Spatial,
  StoreImplementation,
  StoreType,
  StoreZone,
  StringConstraint,
  StringFormat,
  StructDefinition,
  StructType,
  Tenancy,
  Text,
  TextSpan,
  TextSpanType,
  ToolType,
  TraitDefinition,
  TraitType,
  Type,
  TypeCardinality,
  Value,
  Vector2,
  Vector2i,
  Vector3,
  Vector3i,
  Vector4,
  Vector4i,
} from "@destack/language/core";
import type { File, FileFormat, FileRetentionMode, FileSource, FileType, Link, LinkType } from "@destack/language/data";
import type { Environment } from "@destack/language/deployment";
import type { Folder, FolderType, Tag, Tagging } from "@destack/language/folder";
import type { Database, DatabaseInfo, DatabaseType, GalaxyInfo, Machine, MachineType } from "@destack/language/infra";
import type { ModelDeveloper, ModelProvider } from "@destack/language/intelligence";
import type {
  Action,
  ActionCardinality,
  Cursor,
  CursorStatus,
  DayOfWeek,
  EventCursor,
  Month,
  Route,
  Schedule,
  ScheduleFrequency,
  ScreenCursor,
  Script,
  Service,
  ThreadCursor,
  Timer,
  TimerEvent,
  TimerEventType,
  TimerType,
  Trigger,
  TriggerEvent,
  TriggerEventType,
  TriggerType,
} from "@destack/language/logic";
import type {
  Interruption,
  InterruptionResponse,
  InterruptionStatus,
  InterruptionType,
  Log,
  LogLevel,
  Run,
  RunEvent,
  RunEventType,
  RunStatus,
  Span,
} from "@destack/language/runtime";
import type {
  Layer,
  LayerType,
  Scene,
  SceneEvent,
  SceneEventType,
  Variant,
  VariantStateType,
  VariantType,
  Window,
  WindowType,
} from "@destack/language/scene";
import type {
  Follow,
  Message,
  Notification,
  NotificationEvent,
  NotificationEventType,
  NotificationStatus,
  Reaction,
  Star,
  Thread,
  ThreadStatus,
} from "@destack/language/social";
import type {
  Agent,
  Client,
  Friendship,
  FriendshipInvite,
  FriendshipInviteEvent,
  FriendshipInviteEventType,
  Handle,
  Organization,
  OrganizationStatus,
  Origin,
  Space,
  SpaceStatus,
  Team,
  User,
  UserStatus,
} from "@destack/language/space";
import type {
  Border,
  BorderStyle,
  BorderType,
  Color,
  ColorHue,
  ColorIntent,
  ColorShade,
  ColorStyle,
  ColorType,
  Effect,
  EffectStyle,
  EffectType,
  Fill,
  FillPosition,
  FillSize,
  FillStyle,
  FillType,
  Font,
  FontSize,
  FontStyle,
  FontType,
  FontWeight,
  Gradient,
  GradientStop,
  GradientStyle,
  GradientType,
  OffscreenBehavior,
  Palette,
  RepeatType,
  Shadow,
  ShadowPosition,
  ShadowStyle,
  ShadowType,
  SpringType,
  Style,
  TextAlign,
  TextDecoration,
  TextSplitType,
  TextTransform,
  Theme,
  Transition,
  TransitionStyle,
  TransitionType,
} from "@destack/language/style";
import type {
  ContainerView,
  ContentView,
  CustomView,
  CustomViewDefinition,
  FrameView,
  InputView,
  InternalView,
  LabelView,
  NodeView,
  NumberInputView,
  SliderInputView,
  SplitView,
  TextView,
  ThreadView,
  View,
  WizardView,
} from "@destack/language/view";

export type NodeTypeMapping = {
  [NodeType.CUSTOM_ENTITY_DEFINITION]: CustomEntityDefinition;
  [NodeType.CUSTOM_ENTITY]: CustomEntity;
  [NodeType.CUSTOM_ENUM_DEFINITION]: CustomEnumDefinition;
  [NodeType.EDIT_EVENT]: EditEvent;
  [NodeType.CUSTOM_EVENT_DEFINITION]: CustomEventDefinition;
  [NodeType.CUSTOM_EVENT]: CustomEvent;
  [NodeType.GAUGE_METRIC]: GaugeMetric;
  [NodeType.GAUGE_MEASUREMENT]: GaugeMeasurement;
  [NodeType.COUNTER_METRIC]: CounterMetric;
  [NodeType.COUNTER_MEASUREMENT]: CounterMeasurement;
  [NodeType.HISTOGRAM_METRIC]: HistogramMetric;
  [NodeType.HISTOGRAM_MEASUREMENT]: HistogramMeasurement;
  [NodeType.CUSTOM_OPTION]: CustomOption;
  [NodeType.CUSTOM_PROPERTY]: CustomProperty;
  [NodeType.SNAPSHOT]: Snapshot;
  [NodeType.BRANCH]: Branch;
  [NodeType.CUSTOM_STRUCT_DEFINITION]: CustomStructDefinition;
  [NodeType.ENTITLEMENT_EVENT]: EntitlementEvent;
  [NodeType.ENTITLEMENT]: Entitlement;
  [NodeType.INVITE_EVENT]: InviteEvent;
  [NodeType.INVITE]: Invite;
  [NodeType.MEMBERSHIP_EVENT]: MembershipEvent;
  [NodeType.MEMBERSHIP]: Membership;
  [NodeType.PERMISSION]: Permission;
  [NodeType.ROLE_EVENT]: RoleEvent;
  [NodeType.ROLE]: Role;
  [NodeType.SANCTION_EVENT]: SanctionEvent;
  [NodeType.SANCTION]: Sanction;
  [NodeType.CUSTOM_VIEW_DEFINITION]: CustomViewDefinition;
  [NodeType.CUSTOM_VIEW]: CustomView;
  [NodeType.FRAME_VIEW]: FrameView;
  [NodeType.LABEL_VIEW]: LabelView;
  [NodeType.SPLIT_VIEW]: SplitView;
  [NodeType.TEXT_VIEW]: TextView;
  [NodeType.NUMBER_INPUT_VIEW]: NumberInputView;
  [NodeType.SLIDER_INPUT_VIEW]: SliderInputView;
  [NodeType.WIZARD_VIEW]: WizardView;
  [NodeType.THREAD_VIEW]: ThreadView;
  [NodeType.ANNOTATION_SHAPE]: AnnotationShape;
  [NodeType.ARROW_SHAPE]: ArrowShape;
  [NodeType.CANVAS]: Canvas;
  [NodeType.LINE_SHAPE]: LineShape;
  [NodeType.PLANE_SHAPE]: PlaneShape;
  [NodeType.FILE]: File;
  [NodeType.LINK]: Link;
  [NodeType.ENVIRONMENT]: Environment;
  [NodeType.FOLDER]: Folder;
  [NodeType.TAG]: Tag;
  [NodeType.TAGGING]: Tagging;
  [NodeType.DATABASE]: Database;
  [NodeType.MACHINE]: Machine;
  [NodeType.ACTION]: Action;
  [NodeType.EVENT_CURSOR]: EventCursor;
  [NodeType.SCREEN_CURSOR]: ScreenCursor;
  [NodeType.THREAD_CURSOR]: ThreadCursor;
  [NodeType.ROUTE]: Route;
  [NodeType.SCRIPT]: Script;
  [NodeType.SERVICE]: Service;
  [NodeType.TIMER_EVENT]: TimerEvent;
  [NodeType.TIMER]: Timer;
  [NodeType.TRIGGER_EVENT]: TriggerEvent;
  [NodeType.TRIGGER]: Trigger;
  [NodeType.INTERRUPTION]: Interruption;
  [NodeType.LOG]: Log;
  [NodeType.RUN_EVENT]: RunEvent;
  [NodeType.RUN]: Run;
  [NodeType.SPAN]: Span;
  [NodeType.LAYER]: Layer;
  [NodeType.SCENE_EVENT]: SceneEvent;
  [NodeType.SCENE]: Scene;
  [NodeType.VARIANT]: Variant;
  [NodeType.WINDOW]: Window;
  [NodeType.FOLLOW]: Follow;
  [NodeType.MESSAGE]: Message;
  [NodeType.NOTIFICATION_EVENT]: NotificationEvent;
  [NodeType.NOTIFICATION]: Notification;
  [NodeType.REACTION]: Reaction;
  [NodeType.STAR]: Star;
  [NodeType.THREAD]: Thread;
  [NodeType.AGENT]: Agent;
  [NodeType.CLIENT]: Client;
  [NodeType.FRIENDSHIP]: Friendship;
  [NodeType.FRIENDSHIP_INVITE_EVENT]: FriendshipInviteEvent;
  [NodeType.FRIENDSHIP_INVITE]: FriendshipInvite;
  [NodeType.HANDLE]: Handle;
  [NodeType.ORGANIZATION]: Organization;
  [NodeType.SPACE]: Space;
  [NodeType.TEAM]: Team;
  [NodeType.USER]: User;
  [NodeType.COLOR_STYLE]: ColorStyle;
  [NodeType.BORDER_STYLE]: BorderStyle;
  [NodeType.TRANSITION_STYLE]: TransitionStyle;
  [NodeType.EFFECT_STYLE]: EffectStyle;
  [NodeType.GRADIENT_STYLE]: GradientStyle;
  [NodeType.FILL_STYLE]: FillStyle;
  [NodeType.FONT_STYLE]: FontStyle;
  [NodeType.PALETTE]: Palette;
  [NodeType.SHADOW_STYLE]: ShadowStyle;
  [NodeType.THEME]: Theme;
};

export type TraitTypeMapping = {
  [TraitType.HAS_NAME]: HasName;
  [TraitType.HAS_SLUG]: HasSlug;
  [TraitType.HAS_ICON]: HasIcon;
  [TraitType.TRACKED]: IsTracked;
  [TraitType.VISUAL]: IsVisual;
  [TraitType.FROZEN]: IsFrozen;
  [TraitType.ARCHIVABLE]: IsArchivable;
  [TraitType.DELETABLE]: IsDeletable;
  [TraitType.CUSTOM_NODE_DEFINITION]: IsCustomNodeDefinition;
  [TraitType.CUSTOM_NODE]: IsCustomNode;
  [TraitType.EXTENSIBLE]: IsExtensible;
  [TraitType.ORDERED]: IsOrdered;
  [TraitType.REACTABLE]: IsReactable;
  [TraitType.STARABLE]: IsStarable;
  [TraitType.FOLLOWABLE]: IsFollowable;
  [TraitType.SOURCEABLE]: IsSourceable;
  [TraitType.SCRIPTABLE]: IsScriptable;
  [TraitType.RUNNABLE]: IsRunnable;
  [TraitType.ACTIONABLE]: IsActionable;
  [TraitType.OWNABLE]: IsOwnable;
  [TraitType.SETTINGS]: IsSettings;
  [TraitType.JOINABLE]: IsJoinable;
  [TraitType.SUBJECT]: IsSubject;
  [TraitType.OWNER]: IsOwner;
  [TraitType.TAGGABLE]: IsTaggable;
  [TraitType.MEMBERSHIP]: LikeMembership;
  [TraitType.INVITE]: LikeInvite;
  [TraitType.TAG]: LikeTag;
  [TraitType.FOLLOW]: LikeFollow;
  [TraitType.GLOBAL]: Global;
  [TraitType.SPATIAL]: Spatial;
  [TraitType.ENTITY]: Entity;
  [TraitType.PARTICLE]: Particle;
  [TraitType.ANALYTIC]: Analytic;
  [TraitType.INDEXED]: Indexed;
  [TraitType.RESOURCE]: Resource;
  [TraitType.METRIC]: Metric;
  [TraitType.MEASUREMENT]: Measurement;
  [TraitType.EVENT]: Event;
  [TraitType.VIEW]: View;
  [TraitType.CONTAINER_VIEW]: ContainerView;
  [TraitType.CONTENT_VIEW]: ContentView;
  [TraitType.INPUT_VIEW]: InputView;
  [TraitType.INTERNAL_VIEW]: InternalView;
  [TraitType.NODE_VIEW]: NodeView;
  [TraitType.SHAPE]: IsShape;
  [TraitType.CURSOR]: Cursor;
  [TraitType.STYLE]: Style;
};

export type StructTypeMapping = {
  [StructType.SCOPE]: Scope;
  [StructType.RELATION_REFERENCE]: RelationReference;
  [StructType.OBJECT_REFERENCE]: ObjectReference;
  [StructType.PROPERTY_REFERENCE]: PropertyReference;
  [StructType.NODE_REFERENCE]: NodeReference;
  [StructType.EDIT]: Edit;
  [StructType.CHANGE]: Change;
  [StructType.CHANGE_RESULT]: ChangeResult;
  [StructType.ICON]: Icon;
  [StructType.STRING_CONSTRAINT]: StringConstraint;
  [StructType.NUMBER_CONSTRAINT]: NumberConstraint;
  [StructType.COLLECTION_CONSTRAINT]: CollectionConstraint;
  [StructType.NODE_CONSTRAINT]: NodeConstraint;
  [StructType.TYPE]: Type;
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition;
  [StructType.TRAIT_DEFINITION]: TraitDefinition;
  [StructType.NODE_DEFINITION]: NodeDefinition;
  [StructType.STRUCT_DEFINITION]: StructDefinition;
  [StructType.ENUM_DEFINITION]: EnumDefinition;
  [StructType.OPTION_DEFINITION]: OptionDefinition;
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition;
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition;
  [StructType.VALUE]: Value;
  [StructType.FUNCTION]: Function;
  [StructType.CONDITION]: Condition;
  [StructType.AGGREGATION]: Aggregation;
  [StructType.EXPRESSION]: Expression;
  [StructType.SORT]: Sort;
  [StructType.SELECT]: Select;
  [StructType.JOIN]: Join;
  [StructType.QUERY]: Query;
  [StructType.HISTOGRAM]: Histogram;
  [StructType.QUERY_RESULT]: QueryResult;
  [StructType.QUERY_RESULT_GROUP]: QueryResultGroup;
  [StructType.QUERY_UPDATE]: QueryUpdate;
  [StructType.SELECTION]: Selection;
  [StructType.TEXT_SPAN]: TextSpan;
  [StructType.TEXT]: Text;
  [StructType.LENGTH]: Length;
  [StructType.POSITION]: Position;
  [StructType.DIMENSION]: Dimension;
  [StructType.INSETS]: Insets;
  [StructType.CORNERS]: Corners;
  [StructType.AXIS2]: Axis2;
  [StructType.AXIS3]: Axis3;
  [StructType.VECTOR2]: Vector2;
  [StructType.VECTOR3]: Vector3;
  [StructType.VECTOR4]: Vector4;
  [StructType.VECTOR2I]: Vector2i;
  [StructType.VECTOR3I]: Vector3i;
  [StructType.VECTOR4I]: Vector4i;
  [StructType.GRID]: Grid;
  [StructType.GRID_SPAN]: GridSpan;
  [StructType.DATABASE_INFO]: DatabaseInfo;
  [StructType.GALAXY_INFO]: GalaxyInfo;
  [StructType.SCHEDULE]: Schedule;
  [StructType.ORIGIN]: Origin;
  [StructType.COLOR]: Color;
  [StructType.BORDER]: Border;
  [StructType.TRANSITION]: Transition;
  [StructType.EFFECT]: Effect;
  [StructType.GRADIENT_STOP]: GradientStop;
  [StructType.GRADIENT]: Gradient;
  [StructType.FILL]: Fill;
  [StructType.FONT]: Font;
  [StructType.SHADOW]: Shadow;
};

export type EnumTypeMapping = {
  [EnumType.ENUM_TYPE]: EnumType;
  [EnumType.STRUCT_TYPE]: StructType;
  [EnumType.NODE_TYPE]: NodeType;
  [EnumType.TRAIT_TYPE]: TraitType;
  [EnumType.STORE_ZONE]: StoreZone;
  [EnumType.STORE_TYPE]: StoreType;
  [EnumType.STORE_IMPLEMENTATION]: StoreImplementation;
  [EnumType.RUNTIME_TYPE]: RuntimeType;
  [EnumType.PLATFORM_TYPE]: PlatformType;
  [EnumType.OPERATING_SYSTEM]: OperatingSystem;
  [EnumType.ENVIRONMENT_TYPE]: EnvironmentType;
  [EnumType.NODE_PERMISSION]: NodePermission;
  [EnumType.MATERIALIZATION_TYPE]: MaterializationType;
  [EnumType.MODE_TYPE]: ModeType;
  [EnumType.TOOL_TYPE]: ToolType;
  [EnumType.CLOUD]: Cloud;
  [EnumType.REGION_CONTINENT]: RegionContinent;
  [EnumType.REGION_AREA]: RegionArea;
  [EnumType.REGION]: Region;
  [EnumType.EDGE_TYPE]: EdgeType;
  [EnumType.CASCADE_ACTION]: CascadeAction;
  [EnumType.EDGE_DIRECTION]: EdgeDirection;
  [EnumType.PRIMITIVE_TYPE]: PrimitiveType;
  [EnumType.TYPE_CARDINALITY]: TypeCardinality;
  [EnumType.SCALAR_TYPE]: ScalarType;
  [EnumType.DEFAULT_FACTORY]: DefaultFactory;
  [EnumType.ROLE_TYPE]: RoleType;
  [EnumType.RESOURCE_STATUS]: ResourceStatus;
  [EnumType.CLIENT_TYPE]: ClientType;
  [EnumType.TENANCY]: Tenancy;
  [EnumType.JOINABLE_PERMISSION]: JoinablePermission;
  [EnumType.RELATION_TYPE]: RelationType;
  [EnumType.OBJECT_TYPE]: ObjectType;
  [EnumType.PROPERTY_REFERENCE_TYPE]: PropertyReferenceType;
  [EnumType.EDIT_TYPE]: EditType;
  [EnumType.EDIT_OPERATION]: EditOperation;
  [EnumType.CHANGE_STATUS]: ChangeStatus;
  [EnumType.CHANGE_DEBOUNCE]: ChangeDebounce;
  [EnumType.ICON_TYPE]: IconType;
  [EnumType.STRING_FORMAT]: StringFormat;
  [EnumType.NUMBER_FORMAT]: NumberFormat;
  [EnumType.FUNCTION_TYPE]: FunctionType;
  [EnumType.CONDITIONAL_TYPE]: ConditionalType;
  [EnumType.AGGREGATION_TYPE]: AggregationType;
  [EnumType.EXPRESSION_TYPE]: ExpressionType;
  [EnumType.SORT_TYPE]: SortType;
  [EnumType.SORT_MODE]: SortMode;
  [EnumType.JOIN_TYPE]: JoinType;
  [EnumType.QUERY_TYPE]: QueryType;
  [EnumType.QUERY_UPDATE_TYPE]: QueryUpdateType;
  [EnumType.CUSTOM_PROPERTY_TYPE]: CustomPropertyType;
  [EnumType.TEXT_SPAN_TYPE]: TextSpanType;
  [EnumType.LAYOUT]: Layout;
  [EnumType.OVERFLOW]: Overflow;
  [EnumType.DIRECTION]: Direction;
  [EnumType.DISTRIBUTE]: Distribute;
  [EnumType.ALIGN]: Align;
  [EnumType.LENGTH_UNIT]: LengthUnit;
  [EnumType.POSITION_TYPE]: PositionType;
  [EnumType.DIMENSION_TYPE]: DimensionType;
  [EnumType.ENTITLEMENT_EVENT_TYPE]: EntitlementEventType;
  [EnumType.ENTITLEMENT_TYPE]: EntitlementType;
  [EnumType.INVITE_EVENT_TYPE]: InviteEventType;
  [EnumType.MEMBERSHIP_EVENT_TYPE]: MembershipEventType;
  [EnumType.MEMBERSHIP_PERMISSION]: MembershipPermission;
  [EnumType.PERMISSION_TYPE]: PermissionType;
  [EnumType.ROLE_EVENT_TYPE]: RoleEventType;
  [EnumType.SANCTION_EVENT_TYPE]: SanctionEventType;
  [EnumType.SANCTION_TYPE]: SanctionType;
  [EnumType.ARROW_HEAD_TYPE]: ArrowHeadType;
  [EnumType.CANVAS_TYPE]: CanvasType;
  [EnumType.LINE_TYPE]: LineType;
  [EnumType.PLANE_SHAPE_TYPE]: PlaneShapeType;
  [EnumType.FILE_SOURCE]: FileSource;
  [EnumType.FILE_RETENTION_MODE]: FileRetentionMode;
  [EnumType.FILE_TYPE]: FileType;
  [EnumType.FILE_FORMAT]: FileFormat;
  [EnumType.LINK_TYPE]: LinkType;
  [EnumType.FOLDER_TYPE]: FolderType;
  [EnumType.DATABASE_TYPE]: DatabaseType;
  [EnumType.MACHINE_TYPE]: MachineType;
  [EnumType.MODEL_DEVELOPER]: ModelDeveloper;
  [EnumType.MODEL_PROVIDER]: ModelProvider;
  [EnumType.ACTION_CARDINALITY]: ActionCardinality;
  [EnumType.CURSOR_STATUS]: CursorStatus;
  [EnumType.DAY_OF_WEEK]: DayOfWeek;
  [EnumType.MONTH]: Month;
  [EnumType.SCHEDULE_FREQUENCY]: ScheduleFrequency;
  [EnumType.TIMER_EVENT_TYPE]: TimerEventType;
  [EnumType.TIMER_TYPE]: TimerType;
  [EnumType.TRIGGER_EVENT_TYPE]: TriggerEventType;
  [EnumType.TRIGGER_TYPE]: TriggerType;
  [EnumType.INTERRUPTION_TYPE]: InterruptionType;
  [EnumType.INTERRUPTION_STATUS]: InterruptionStatus;
  [EnumType.INTERRUPTION_RESPONSE]: InterruptionResponse;
  [EnumType.LOG_LEVEL]: LogLevel;
  [EnumType.RUN_STATUS]: RunStatus;
  [EnumType.RUN_EVENT_TYPE]: RunEventType;
  [EnumType.LAYER_TYPE]: LayerType;
  [EnumType.SCENE_EVENT_TYPE]: SceneEventType;
  [EnumType.VARIANT_TYPE]: VariantType;
  [EnumType.VARIANT_STATE_TYPE]: VariantStateType;
  [EnumType.WINDOW_TYPE]: WindowType;
  [EnumType.NOTIFICATION_STATUS]: NotificationStatus;
  [EnumType.NOTIFICATION_EVENT_TYPE]: NotificationEventType;
  [EnumType.THREAD_STATUS]: ThreadStatus;
  [EnumType.FRIENDSHIP_INVITE_EVENT_TYPE]: FriendshipInviteEventType;
  [EnumType.ORGANIZATION_STATUS]: OrganizationStatus;
  [EnumType.SPACE_STATUS]: SpaceStatus;
  [EnumType.USER_STATUS]: UserStatus;
  [EnumType.COLOR_TYPE]: ColorType;
  [EnumType.COLOR_HUE]: ColorHue;
  [EnumType.COLOR_SHADE]: ColorShade;
  [EnumType.COLOR_INTENT]: ColorIntent;
  [EnumType.BORDER_TYPE]: BorderType;
  [EnumType.TRANSITION_TYPE]: TransitionType;
  [EnumType.SPRING_TYPE]: SpringType;
  [EnumType.EFFECT_TYPE]: EffectType;
  [EnumType.REPEAT_TYPE]: RepeatType;
  [EnumType.TEXT_SPLIT_TYPE]: TextSplitType;
  [EnumType.OFFSCREEN_BEHAVIOR]: OffscreenBehavior;
  [EnumType.GRADIENT_TYPE]: GradientType;
  [EnumType.FILL_TYPE]: FillType;
  [EnumType.FILL_POSITION]: FillPosition;
  [EnumType.FILL_SIZE]: FillSize;
  [EnumType.FONT_TYPE]: FontType;
  [EnumType.FONT_WEIGHT]: FontWeight;
  [EnumType.FONT_SIZE]: FontSize;
  [EnumType.TEXT_ALIGN]: TextAlign;
  [EnumType.TEXT_DECORATION]: TextDecoration;
  [EnumType.TEXT_TRANSFORM]: TextTransform;
  [EnumType.SHADOW_TYPE]: ShadowType;
  [EnumType.SHADOW_POSITION]: ShadowPosition;
};
