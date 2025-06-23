import {
  Action,
  ActionCardinality,
  Agent,
  Aggregation,
  AggregationType,
  Align,
  Analytic,
  AnnotationShape,
  ArrowHeadType,
  ArrowShape,
  AttributeReference,
  AttributeType,
  Axis2,
  Axis3,
  Border,
  BorderStyle,
  BorderType,
  Branch,
  Canvas,
  CanvasType,
  CascadeAction,
  Change,
  ChangeDebounce,
  ChangeResult,
  ChangeStatus,
  Client,
  ClientType,
  Cloud,
  CollectionConstraint,
  Color,
  ColorHue,
  ColorIntent,
  ColorShade,
  ColorStyle,
  ColorType,
  Condition,
  ConditionalType,
  ConstantDefinition,
  ContainerView,
  ContentView,
  Corners,
  CounterMeasurement,
  CounterMetric,
  Cursor,
  CursorStatus,
  CustomEntity,
  CustomEntityDefinition,
  CustomEnumDefinition,
  CustomEvent,
  CustomEventDefinition,
  CustomStructDefinition,
  CustomView,
  CustomViewDefinition,
  Database,
  DatabaseInfo,
  DatabaseType,
  DayOfWeek,
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
  Effect,
  EffectStyle,
  EffectType,
  Entitlement,
  EntitlementEvent,
  EntitlementEventType,
  EntitlementType,
  Entity,
  EnumDefinition,
  EnumOptionDefinition,
  EnumType,
  Environment,
  EnvironmentType,
  Event,
  EventCursor,
  Expression,
  ExpressionType,
  Field,
  FieldType,
  File,
  FileFormat,
  FileRetentionMode,
  FileSource,
  FileType,
  Fill,
  FillPosition,
  FillSize,
  FillStyle,
  FillType,
  Folder,
  FolderType,
  Follow,
  Font,
  FontSize,
  FontStyle,
  FontType,
  FontWeight,
  FrameView,
  Friendship,
  FriendshipInvite,
  FriendshipInviteEvent,
  FriendshipInviteEventType,
  Function,
  FunctionType,
  GalaxyInfo,
  GaugeMeasurement,
  GaugeMetric,
  Global,
  Gradient,
  GradientStop,
  GradientStyle,
  GradientType,
  Grid,
  GridSpan,
  Handle,
  HasIcon,
  HasName,
  HasSlug,
  Histogram,
  HistogramMeasurement,
  HistogramMetric,
  Icon,
  IconType,
  Indexed,
  InputView,
  Insets,
  InternalView,
  Interruption,
  InterruptionResponse,
  InterruptionStatus,
  InterruptionType,
  Invite,
  InviteEvent,
  InviteEventType,
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
  IsShape,
  IsSourceable,
  IsStarable,
  IsSubject,
  IsTaggable,
  IsTracked,
  IsVisual,
  Join,
  JoinablePermission,
  JoinType,
  LabelView,
  Layer,
  LayerType,
  Layout,
  Length,
  LengthUnit,
  LikeFollow,
  LikeInvite,
  LikeMembership,
  LikeTag,
  LineShape,
  LineType,
  Link,
  LinkType,
  Log,
  LogLevel,
  Machine,
  MachineType,
  MaterializationType,
  Measurement,
  Membership,
  MembershipEvent,
  MembershipEventType,
  MembershipPermission,
  Message,
  Metric,
  ModelDeveloper,
  ModelProvider,
  ModeType,
  Month,
  NodeConstraint,
  NodeDefinition,
  NodePermission,
  NodeReference,
  NodeType,
  NodeView,
  Notification,
  NotificationEvent,
  NotificationEventType,
  NotificationStatus,
  NumberConstraint,
  NumberFormat,
  NumberInputView,
  OffscreenBehavior,
  OperatingSystem,
  Option,
  Organization,
  OrganizationStatus,
  Origin,
  Overflow,
  Palette,
  Particle,
  Permission,
  PermissionDefinition,
  PermissionType,
  PlaneShape,
  PlaneShapeType,
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
  Reaction,
  Region,
  RegionArea,
  RegionContinent,
  RelationReference,
  RelationType,
  RepeatType,
  Resource,
  ResourceStatus,
  Role,
  RoleEvent,
  RoleEventType,
  RoleType,
  Route,
  Run,
  RunEvent,
  RunEventType,
  RunStatus,
  RuntimeType,
  Sanction,
  SanctionEvent,
  SanctionEventType,
  SanctionType,
  ScalarType,
  Scene,
  SceneEvent,
  SceneEventType,
  Schedule,
  ScheduleFrequency,
  Scope,
  ScreenCursor,
  Script,
  Select,
  Selection,
  Service,
  Shadow,
  ShadowPosition,
  ShadowStyle,
  ShadowType,
  SliderInputView,
  Snapshot,
  Sort,
  SortMode,
  SortType,
  Space,
  SpaceStatus,
  Span,
  Spatial,
  SplitView,
  SpringType,
  Star,
  StoreImplementation,
  StoreType,
  StoreZone,
  StringConstraint,
  StringFormat,
  StructDefinition,
  StructType,
  Style,
  Tag,
  Tagging,
  Team,
  Tenancy,
  Text,
  TextAlign,
  TextDecoration,
  TextSpan,
  TextSpanType,
  TextSplitType,
  TextTransform,
  TextView,
  Theme,
  Thread,
  ThreadCursor,
  ThreadStatus,
  ThreadView,
  Timer,
  TimerEvent,
  TimerEventType,
  TimerType,
  ToolType,
  TraitDefinition,
  TraitType,
  Transition,
  TransitionStyle,
  TransitionType,
  Trigger,
  TriggerEvent,
  TriggerEventType,
  TriggerType,
  Type,
  TypeCardinality,
  User,
  UserStatus,
  Value,
  Variant,
  VariantStateType,
  VariantType,
  Vector2,
  Vector2i,
  Vector3,
  Vector3i,
  Vector4,
  Vector4i,
  View,
  Window,
  WindowType,
  WizardView,
} from "@/language";

export type NodeTypeMapping = {
  [NodeType.CUSTOM_ENTITY_DEFINITION]: CustomEntityDefinition;
  [NodeType.CUSTOM_ENTITY]: CustomEntity;
  [NodeType.CUSTOM_ENUM_DEFINITION]: CustomEnumDefinition;
  [NodeType.EDIT_EVENT]: EditEvent;
  [NodeType.CUSTOM_EVENT_DEFINITION]: CustomEventDefinition;
  [NodeType.CUSTOM_EVENT]: CustomEvent;
  [NodeType.FIELD]: Field;
  [NodeType.GAUGE_METRIC]: GaugeMetric;
  [NodeType.GAUGE_MEASUREMENT]: GaugeMeasurement;
  [NodeType.COUNTER_METRIC]: CounterMetric;
  [NodeType.COUNTER_MEASUREMENT]: CounterMeasurement;
  [NodeType.HISTOGRAM_METRIC]: HistogramMetric;
  [NodeType.HISTOGRAM_MEASUREMENT]: HistogramMeasurement;
  [NodeType.OPTION]: Option;
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

export const NODE_CLASS_BY_TYPE = {
  [NodeType.CUSTOM_ENTITY_DEFINITION]: CustomEntityDefinition,
  [NodeType.CUSTOM_ENTITY]: CustomEntity,
  [NodeType.CUSTOM_ENUM_DEFINITION]: CustomEnumDefinition,
  [NodeType.EDIT_EVENT]: EditEvent,
  [NodeType.CUSTOM_EVENT_DEFINITION]: CustomEventDefinition,
  [NodeType.CUSTOM_EVENT]: CustomEvent,
  [NodeType.FIELD]: Field,
  [NodeType.GAUGE_METRIC]: GaugeMetric,
  [NodeType.GAUGE_MEASUREMENT]: GaugeMeasurement,
  [NodeType.COUNTER_METRIC]: CounterMetric,
  [NodeType.COUNTER_MEASUREMENT]: CounterMeasurement,
  [NodeType.HISTOGRAM_METRIC]: HistogramMetric,
  [NodeType.HISTOGRAM_MEASUREMENT]: HistogramMeasurement,
  [NodeType.OPTION]: Option,
  [NodeType.SNAPSHOT]: Snapshot,
  [NodeType.BRANCH]: Branch,
  [NodeType.CUSTOM_STRUCT_DEFINITION]: CustomStructDefinition,
  [NodeType.ENTITLEMENT_EVENT]: EntitlementEvent,
  [NodeType.ENTITLEMENT]: Entitlement,
  [NodeType.INVITE_EVENT]: InviteEvent,
  [NodeType.INVITE]: Invite,
  [NodeType.MEMBERSHIP_EVENT]: MembershipEvent,
  [NodeType.MEMBERSHIP]: Membership,
  [NodeType.PERMISSION]: Permission,
  [NodeType.ROLE_EVENT]: RoleEvent,
  [NodeType.ROLE]: Role,
  [NodeType.SANCTION_EVENT]: SanctionEvent,
  [NodeType.SANCTION]: Sanction,
  [NodeType.CUSTOM_VIEW_DEFINITION]: CustomViewDefinition,
  [NodeType.CUSTOM_VIEW]: CustomView,
  [NodeType.FRAME_VIEW]: FrameView,
  [NodeType.LABEL_VIEW]: LabelView,
  [NodeType.SPLIT_VIEW]: SplitView,
  [NodeType.TEXT_VIEW]: TextView,
  [NodeType.NUMBER_INPUT_VIEW]: NumberInputView,
  [NodeType.SLIDER_INPUT_VIEW]: SliderInputView,
  [NodeType.WIZARD_VIEW]: WizardView,
  [NodeType.THREAD_VIEW]: ThreadView,
  [NodeType.ANNOTATION_SHAPE]: AnnotationShape,
  [NodeType.ARROW_SHAPE]: ArrowShape,
  [NodeType.CANVAS]: Canvas,
  [NodeType.LINE_SHAPE]: LineShape,
  [NodeType.PLANE_SHAPE]: PlaneShape,
  [NodeType.FILE]: File,
  [NodeType.LINK]: Link,
  [NodeType.ENVIRONMENT]: Environment,
  [NodeType.FOLDER]: Folder,
  [NodeType.TAG]: Tag,
  [NodeType.TAGGING]: Tagging,
  [NodeType.DATABASE]: Database,
  [NodeType.MACHINE]: Machine,
  [NodeType.ACTION]: Action,
  [NodeType.EVENT_CURSOR]: EventCursor,
  [NodeType.SCREEN_CURSOR]: ScreenCursor,
  [NodeType.THREAD_CURSOR]: ThreadCursor,
  [NodeType.ROUTE]: Route,
  [NodeType.SCRIPT]: Script,
  [NodeType.SERVICE]: Service,
  [NodeType.TIMER_EVENT]: TimerEvent,
  [NodeType.TIMER]: Timer,
  [NodeType.TRIGGER_EVENT]: TriggerEvent,
  [NodeType.TRIGGER]: Trigger,
  [NodeType.INTERRUPTION]: Interruption,
  [NodeType.LOG]: Log,
  [NodeType.RUN_EVENT]: RunEvent,
  [NodeType.RUN]: Run,
  [NodeType.SPAN]: Span,
  [NodeType.LAYER]: Layer,
  [NodeType.SCENE_EVENT]: SceneEvent,
  [NodeType.SCENE]: Scene,
  [NodeType.VARIANT]: Variant,
  [NodeType.WINDOW]: Window,
  [NodeType.FOLLOW]: Follow,
  [NodeType.MESSAGE]: Message,
  [NodeType.NOTIFICATION_EVENT]: NotificationEvent,
  [NodeType.NOTIFICATION]: Notification,
  [NodeType.REACTION]: Reaction,
  [NodeType.STAR]: Star,
  [NodeType.THREAD]: Thread,
  [NodeType.AGENT]: Agent,
  [NodeType.CLIENT]: Client,
  [NodeType.FRIENDSHIP]: Friendship,
  [NodeType.FRIENDSHIP_INVITE_EVENT]: FriendshipInviteEvent,
  [NodeType.FRIENDSHIP_INVITE]: FriendshipInvite,
  [NodeType.HANDLE]: Handle,
  [NodeType.ORGANIZATION]: Organization,
  [NodeType.SPACE]: Space,
  [NodeType.TEAM]: Team,
  [NodeType.USER]: User,
  [NodeType.COLOR_STYLE]: ColorStyle,
  [NodeType.BORDER_STYLE]: BorderStyle,
  [NodeType.TRANSITION_STYLE]: TransitionStyle,
  [NodeType.EFFECT_STYLE]: EffectStyle,
  [NodeType.GRADIENT_STYLE]: GradientStyle,
  [NodeType.FILL_STYLE]: FillStyle,
  [NodeType.FONT_STYLE]: FontStyle,
  [NodeType.PALETTE]: Palette,
  [NodeType.SHADOW_STYLE]: ShadowStyle,
  [NodeType.THEME]: Theme,
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

export const NODE_TYPES_BY_TRAIT_TYPE: Record<TraitType, NodeType[]> = {
  [TraitType.HAS_NAME]: [],
  [TraitType.HAS_SLUG]: [],
  [TraitType.HAS_ICON]: [],
  [TraitType.TRACKED]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.CUSTOM_EVENT,
    NodeType.FIELD,
    NodeType.GAUGE_METRIC,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_METRIC,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.HISTOGRAM_METRIC,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.OPTION,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.ENTITLEMENT,
    NodeType.INVITE_EVENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE_EVENT,
    NodeType.ROLE,
    NodeType.SANCTION_EVENT,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FILE,
    NodeType.LINK,
    NodeType.ENVIRONMENT,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.DATABASE,
    NodeType.MACHINE,
    NodeType.ACTION,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.TIMER_EVENT,
    NodeType.TIMER,
    NodeType.TRIGGER_EVENT,
    NodeType.TRIGGER,
    NodeType.INTERRUPTION,
    NodeType.LOG,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SPAN,
    NodeType.LAYER,
    NodeType.SCENE_EVENT,
    NodeType.SCENE,
    NodeType.VARIANT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.MESSAGE,
    NodeType.NOTIFICATION_EVENT,
    NodeType.NOTIFICATION,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
    NodeType.CLIENT,
    NodeType.FRIENDSHIP,
    NodeType.FRIENDSHIP_INVITE_EVENT,
    NodeType.FRIENDSHIP_INVITE,
    NodeType.HANDLE,
    NodeType.ORGANIZATION,
    NodeType.SPACE,
    NodeType.TEAM,
    NodeType.USER,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.VISUAL]: [
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.WINDOW,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.FROZEN]: [
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.INVITE_EVENT,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.ROLE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.TIMER_EVENT,
    NodeType.TRIGGER_EVENT,
    NodeType.LOG,
    NodeType.RUN_EVENT,
    NodeType.SPAN,
    NodeType.SCENE_EVENT,
    NodeType.NOTIFICATION_EVENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
  ],
  [TraitType.ARCHIVABLE]: [],
  [TraitType.DELETABLE]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ENTITLEMENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ENVIRONMENT,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.ACTION,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.VARIANT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.MESSAGE,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
    NodeType.CLIENT,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.CUSTOM_NODE_DEFINITION]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.GAUGE_METRIC,
    NodeType.COUNTER_METRIC,
    NodeType.HISTOGRAM_METRIC,
    NodeType.CUSTOM_VIEW_DEFINITION,
  ],
  [TraitType.CUSTOM_NODE]: [
    NodeType.CUSTOM_ENTITY,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.CUSTOM_VIEW,
  ],
  [TraitType.EXTENSIBLE]: [
    NodeType.CUSTOM_ENTITY,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CANVAS,
    NodeType.PLANE_SHAPE,
    NodeType.ACTION,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.INTERRUPTION,
    NodeType.RUN,
    NodeType.LAYER,
    NodeType.SCENE,
  ],
  [TraitType.ORDERED]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.FIELD,
    NodeType.GAUGE_METRIC,
    NodeType.COUNTER_METRIC,
    NodeType.HISTOGRAM_METRIC,
    NodeType.OPTION,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ROLE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.ACTION,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.WINDOW,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.REACTABLE]: [NodeType.MESSAGE, NodeType.REACTION],
  [TraitType.STARABLE]: [NodeType.FOLDER, NodeType.SPACE],
  [TraitType.FOLLOWABLE]: [NodeType.FOLDER, NodeType.AGENT, NodeType.SPACE, NodeType.USER],
  [TraitType.SOURCEABLE]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.FIELD,
    NodeType.GAUGE_METRIC,
    NodeType.COUNTER_METRIC,
    NodeType.HISTOGRAM_METRIC,
    NodeType.OPTION,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ACTION,
    NodeType.SERVICE,
  ],
  [TraitType.SCRIPTABLE]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.SERVICE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.AGENT,
  ],
  [TraitType.RUNNABLE]: [NodeType.ACTION, NodeType.SCRIPT, NodeType.SERVICE],
  [TraitType.ACTIONABLE]: [NodeType.CUSTOM_ENTITY_DEFINITION, NodeType.SERVICE],
  [TraitType.OWNABLE]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.FOLDER,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.ROUTE,
    NodeType.SERVICE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.VARIANT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.MESSAGE,
    NodeType.NOTIFICATION,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.FRIENDSHIP_INVITE,
    NodeType.SPACE,
  ],
  [TraitType.SETTINGS]: [],
  [TraitType.JOINABLE]: [NodeType.FOLDER, NodeType.THREAD, NodeType.ORGANIZATION, NodeType.SPACE, NodeType.TEAM],
  [TraitType.SUBJECT]: [NodeType.AGENT, NodeType.USER],
  [TraitType.OWNER]: [NodeType.ROLE, NodeType.AGENT, NodeType.ORGANIZATION, NodeType.TEAM, NodeType.USER],
  [TraitType.TAGGABLE]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.EDIT_EVENT,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FOLDER,
    NodeType.TAGGING,
    NodeType.ACTION,
    NodeType.ROUTE,
    NodeType.SERVICE,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.MESSAGE,
    NodeType.THREAD,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.MEMBERSHIP]: [NodeType.MEMBERSHIP],
  [TraitType.INVITE]: [NodeType.INVITE, NodeType.FRIENDSHIP_INVITE],
  [TraitType.TAG]: [NodeType.TAG],
  [TraitType.FOLLOW]: [NodeType.FOLLOW],
  [TraitType.GLOBAL]: [
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.ROLE,
    NodeType.FILE,
    NodeType.FOLLOW,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.CLIENT,
    NodeType.FRIENDSHIP,
    NodeType.FRIENDSHIP_INVITE,
    NodeType.HANDLE,
    NodeType.ORGANIZATION,
    NodeType.SPACE,
    NodeType.TEAM,
    NodeType.USER,
  ],
  [TraitType.SPATIAL]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.CUSTOM_EVENT,
    NodeType.FIELD,
    NodeType.GAUGE_METRIC,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_METRIC,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.HISTOGRAM_METRIC,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.OPTION,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.ENTITLEMENT,
    NodeType.INVITE_EVENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE_EVENT,
    NodeType.ROLE,
    NodeType.SANCTION_EVENT,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FILE,
    NodeType.LINK,
    NodeType.ENVIRONMENT,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.DATABASE,
    NodeType.MACHINE,
    NodeType.ACTION,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.TIMER_EVENT,
    NodeType.TIMER,
    NodeType.TRIGGER_EVENT,
    NodeType.TRIGGER,
    NodeType.INTERRUPTION,
    NodeType.LOG,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SPAN,
    NodeType.LAYER,
    NodeType.SCENE_EVENT,
    NodeType.SCENE,
    NodeType.VARIANT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.MESSAGE,
    NodeType.NOTIFICATION_EVENT,
    NodeType.NOTIFICATION,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
    NodeType.SPACE,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.ENTITY]: [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.FIELD,
    NodeType.GAUGE_METRIC,
    NodeType.COUNTER_METRIC,
    NodeType.HISTOGRAM_METRIC,
    NodeType.OPTION,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ENTITLEMENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FILE,
    NodeType.LINK,
    NodeType.ENVIRONMENT,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.DATABASE,
    NodeType.MACHINE,
    NodeType.ACTION,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.TIMER,
    NodeType.TRIGGER,
    NodeType.LAYER,
    NodeType.SCENE,
    NodeType.VARIANT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.MESSAGE,
    NodeType.NOTIFICATION,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
    NodeType.CLIENT,
    NodeType.FRIENDSHIP,
    NodeType.FRIENDSHIP_INVITE,
    NodeType.HANDLE,
    NodeType.ORGANIZATION,
    NodeType.SPACE,
    NodeType.TEAM,
    NodeType.USER,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.PALETTE,
    NodeType.SHADOW_STYLE,
    NodeType.THEME,
  ],
  [TraitType.PARTICLE]: [
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.INVITE_EVENT,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.ROLE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.TIMER_EVENT,
    NodeType.TRIGGER_EVENT,
    NodeType.INTERRUPTION,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SCENE_EVENT,
    NodeType.NOTIFICATION_EVENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
  ],
  [TraitType.ANALYTIC]: [
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.INVITE_EVENT,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.ROLE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.TIMER_EVENT,
    NodeType.TRIGGER_EVENT,
    NodeType.INTERRUPTION,
    NodeType.LOG,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SPAN,
    NodeType.SCENE_EVENT,
    NodeType.NOTIFICATION_EVENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
  ],
  [TraitType.INDEXED]: [
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.INVITE_EVENT,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.ROLE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.TIMER_EVENT,
    NodeType.TRIGGER_EVENT,
    NodeType.INTERRUPTION,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SCENE_EVENT,
    NodeType.NOTIFICATION_EVENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
  ],
  [TraitType.RESOURCE]: [NodeType.FILE, NodeType.LINK, NodeType.DATABASE, NodeType.MACHINE],
  [TraitType.METRIC]: [NodeType.GAUGE_METRIC, NodeType.COUNTER_METRIC, NodeType.HISTOGRAM_METRIC],
  [TraitType.MEASUREMENT]: [NodeType.GAUGE_MEASUREMENT, NodeType.COUNTER_MEASUREMENT, NodeType.HISTOGRAM_MEASUREMENT],
  [TraitType.EVENT]: [
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.INVITE_EVENT,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.ROLE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.TIMER_EVENT,
    NodeType.TRIGGER_EVENT,
    NodeType.RUN_EVENT,
    NodeType.SCENE_EVENT,
    NodeType.NOTIFICATION_EVENT,
    NodeType.FRIENDSHIP_INVITE_EVENT,
  ],
  [TraitType.VIEW]: [
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.LAYER,
    NodeType.SCENE,
  ],
  [TraitType.CONTAINER_VIEW]: [
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CANVAS,
    NodeType.PLANE_SHAPE,
    NodeType.LAYER,
    NodeType.SCENE,
  ],
  [TraitType.CONTENT_VIEW]: [NodeType.TEXT_VIEW, NodeType.ARROW_SHAPE, NodeType.LINE_SHAPE],
  [TraitType.INPUT_VIEW]: [NodeType.NUMBER_INPUT_VIEW, NodeType.SLIDER_INPUT_VIEW],
  [TraitType.INTERNAL_VIEW]: [NodeType.WIZARD_VIEW],
  [TraitType.NODE_VIEW]: [NodeType.THREAD_VIEW],
  [TraitType.SHAPE]: [NodeType.ANNOTATION_SHAPE, NodeType.ARROW_SHAPE, NodeType.LINE_SHAPE, NodeType.PLANE_SHAPE],
  [TraitType.CURSOR]: [NodeType.EVENT_CURSOR, NodeType.SCREEN_CURSOR, NodeType.THREAD_CURSOR],
  [TraitType.STYLE]: [
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.SHADOW_STYLE,
  ],
};

export type StructTypeMapping = {
  [StructType.SCOPE]: Scope;
  [StructType.RELATION_REFERENCE]: RelationReference;
  [StructType.ATTRIBUTE_REFERENCE]: AttributeReference;
  [StructType.PROPERTY_REFERENCE]: PropertyReference;
  [StructType.NODE_REFERENCE]: NodeReference;
  [StructType.EDIT]: Edit;
  [StructType.CHANGE]: Change;
  [StructType.CHANGE_RESULT]: ChangeResult;
  [StructType.STRING_CONSTRAINT]: StringConstraint;
  [StructType.NUMBER_CONSTRAINT]: NumberConstraint;
  [StructType.COLLECTION_CONSTRAINT]: CollectionConstraint;
  [StructType.NODE_CONSTRAINT]: NodeConstraint;
  [StructType.TYPE]: Type;
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
  [StructType.ICON]: Icon;
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition;
  [StructType.TRAIT_DEFINITION]: TraitDefinition;
  [StructType.NODE_DEFINITION]: NodeDefinition;
  [StructType.STRUCT_DEFINITION]: StructDefinition;
  [StructType.ENUM_DEFINITION]: EnumDefinition;
  [StructType.ENUM_OPTION_DEFINITION]: EnumOptionDefinition;
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition;
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition;
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

export const STRUCT_CLASS_BY_TYPE = {
  [StructType.SCOPE]: Scope,
  [StructType.RELATION_REFERENCE]: RelationReference,
  [StructType.ATTRIBUTE_REFERENCE]: AttributeReference,
  [StructType.PROPERTY_REFERENCE]: PropertyReference,
  [StructType.NODE_REFERENCE]: NodeReference,
  [StructType.EDIT]: Edit,
  [StructType.CHANGE]: Change,
  [StructType.CHANGE_RESULT]: ChangeResult,
  [StructType.STRING_CONSTRAINT]: StringConstraint,
  [StructType.NUMBER_CONSTRAINT]: NumberConstraint,
  [StructType.COLLECTION_CONSTRAINT]: CollectionConstraint,
  [StructType.NODE_CONSTRAINT]: NodeConstraint,
  [StructType.TYPE]: Type,
  [StructType.VALUE]: Value,
  [StructType.FUNCTION]: Function,
  [StructType.CONDITION]: Condition,
  [StructType.AGGREGATION]: Aggregation,
  [StructType.EXPRESSION]: Expression,
  [StructType.SORT]: Sort,
  [StructType.SELECT]: Select,
  [StructType.JOIN]: Join,
  [StructType.QUERY]: Query,
  [StructType.HISTOGRAM]: Histogram,
  [StructType.QUERY_RESULT]: QueryResult,
  [StructType.QUERY_RESULT_GROUP]: QueryResultGroup,
  [StructType.QUERY_UPDATE]: QueryUpdate,
  [StructType.SELECTION]: Selection,
  [StructType.ICON]: Icon,
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition,
  [StructType.TRAIT_DEFINITION]: TraitDefinition,
  [StructType.NODE_DEFINITION]: NodeDefinition,
  [StructType.STRUCT_DEFINITION]: StructDefinition,
  [StructType.ENUM_DEFINITION]: EnumDefinition,
  [StructType.ENUM_OPTION_DEFINITION]: EnumOptionDefinition,
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition,
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition,
  [StructType.TEXT_SPAN]: TextSpan,
  [StructType.TEXT]: Text,
  [StructType.LENGTH]: Length,
  [StructType.POSITION]: Position,
  [StructType.DIMENSION]: Dimension,
  [StructType.INSETS]: Insets,
  [StructType.CORNERS]: Corners,
  [StructType.AXIS2]: Axis2,
  [StructType.AXIS3]: Axis3,
  [StructType.VECTOR2]: Vector2,
  [StructType.VECTOR3]: Vector3,
  [StructType.VECTOR4]: Vector4,
  [StructType.VECTOR2I]: Vector2i,
  [StructType.VECTOR3I]: Vector3i,
  [StructType.VECTOR4I]: Vector4i,
  [StructType.GRID]: Grid,
  [StructType.GRID_SPAN]: GridSpan,
  [StructType.DATABASE_INFO]: DatabaseInfo,
  [StructType.GALAXY_INFO]: GalaxyInfo,
  [StructType.SCHEDULE]: Schedule,
  [StructType.ORIGIN]: Origin,
  [StructType.COLOR]: Color,
  [StructType.BORDER]: Border,
  [StructType.TRANSITION]: Transition,
  [StructType.EFFECT]: Effect,
  [StructType.GRADIENT_STOP]: GradientStop,
  [StructType.GRADIENT]: Gradient,
  [StructType.FILL]: Fill,
  [StructType.FONT]: Font,
  [StructType.SHADOW]: Shadow,
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
  [EnumType.ATTRIBUTE_TYPE]: AttributeType;
  [EnumType.PROPERTY_REFERENCE_TYPE]: PropertyReferenceType;
  [EnumType.EDIT_TYPE]: EditType;
  [EnumType.EDIT_OPERATION]: EditOperation;
  [EnumType.CHANGE_STATUS]: ChangeStatus;
  [EnumType.CHANGE_DEBOUNCE]: ChangeDebounce;
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
  [EnumType.FIELD_TYPE]: FieldType;
  [EnumType.ICON_TYPE]: IconType;
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

export const ENUM_CLASS_BY_TYPE = {
  [EnumType.ENUM_TYPE]: EnumType,
  [EnumType.STRUCT_TYPE]: StructType,
  [EnumType.NODE_TYPE]: NodeType,
  [EnumType.TRAIT_TYPE]: TraitType,
  [EnumType.STORE_ZONE]: StoreZone,
  [EnumType.STORE_TYPE]: StoreType,
  [EnumType.STORE_IMPLEMENTATION]: StoreImplementation,
  [EnumType.RUNTIME_TYPE]: RuntimeType,
  [EnumType.PLATFORM_TYPE]: PlatformType,
  [EnumType.OPERATING_SYSTEM]: OperatingSystem,
  [EnumType.ENVIRONMENT_TYPE]: EnvironmentType,
  [EnumType.NODE_PERMISSION]: NodePermission,
  [EnumType.MATERIALIZATION_TYPE]: MaterializationType,
  [EnumType.MODE_TYPE]: ModeType,
  [EnumType.TOOL_TYPE]: ToolType,
  [EnumType.CLOUD]: Cloud,
  [EnumType.REGION_CONTINENT]: RegionContinent,
  [EnumType.REGION_AREA]: RegionArea,
  [EnumType.REGION]: Region,
  [EnumType.EDGE_TYPE]: EdgeType,
  [EnumType.CASCADE_ACTION]: CascadeAction,
  [EnumType.EDGE_DIRECTION]: EdgeDirection,
  [EnumType.PRIMITIVE_TYPE]: PrimitiveType,
  [EnumType.TYPE_CARDINALITY]: TypeCardinality,
  [EnumType.SCALAR_TYPE]: ScalarType,
  [EnumType.DEFAULT_FACTORY]: DefaultFactory,
  [EnumType.ROLE_TYPE]: RoleType,
  [EnumType.RESOURCE_STATUS]: ResourceStatus,
  [EnumType.CLIENT_TYPE]: ClientType,
  [EnumType.TENANCY]: Tenancy,
  [EnumType.JOINABLE_PERMISSION]: JoinablePermission,
  [EnumType.RELATION_TYPE]: RelationType,
  [EnumType.ATTRIBUTE_TYPE]: AttributeType,
  [EnumType.PROPERTY_REFERENCE_TYPE]: PropertyReferenceType,
  [EnumType.EDIT_TYPE]: EditType,
  [EnumType.EDIT_OPERATION]: EditOperation,
  [EnumType.CHANGE_STATUS]: ChangeStatus,
  [EnumType.CHANGE_DEBOUNCE]: ChangeDebounce,
  [EnumType.STRING_FORMAT]: StringFormat,
  [EnumType.NUMBER_FORMAT]: NumberFormat,
  [EnumType.FUNCTION_TYPE]: FunctionType,
  [EnumType.CONDITIONAL_TYPE]: ConditionalType,
  [EnumType.AGGREGATION_TYPE]: AggregationType,
  [EnumType.EXPRESSION_TYPE]: ExpressionType,
  [EnumType.SORT_TYPE]: SortType,
  [EnumType.SORT_MODE]: SortMode,
  [EnumType.JOIN_TYPE]: JoinType,
  [EnumType.QUERY_TYPE]: QueryType,
  [EnumType.QUERY_UPDATE_TYPE]: QueryUpdateType,
  [EnumType.FIELD_TYPE]: FieldType,
  [EnumType.ICON_TYPE]: IconType,
  [EnumType.TEXT_SPAN_TYPE]: TextSpanType,
  [EnumType.LAYOUT]: Layout,
  [EnumType.OVERFLOW]: Overflow,
  [EnumType.DIRECTION]: Direction,
  [EnumType.DISTRIBUTE]: Distribute,
  [EnumType.ALIGN]: Align,
  [EnumType.LENGTH_UNIT]: LengthUnit,
  [EnumType.POSITION_TYPE]: PositionType,
  [EnumType.DIMENSION_TYPE]: DimensionType,
  [EnumType.ENTITLEMENT_EVENT_TYPE]: EntitlementEventType,
  [EnumType.ENTITLEMENT_TYPE]: EntitlementType,
  [EnumType.INVITE_EVENT_TYPE]: InviteEventType,
  [EnumType.MEMBERSHIP_EVENT_TYPE]: MembershipEventType,
  [EnumType.MEMBERSHIP_PERMISSION]: MembershipPermission,
  [EnumType.PERMISSION_TYPE]: PermissionType,
  [EnumType.ROLE_EVENT_TYPE]: RoleEventType,
  [EnumType.SANCTION_EVENT_TYPE]: SanctionEventType,
  [EnumType.SANCTION_TYPE]: SanctionType,
  [EnumType.ARROW_HEAD_TYPE]: ArrowHeadType,
  [EnumType.CANVAS_TYPE]: CanvasType,
  [EnumType.LINE_TYPE]: LineType,
  [EnumType.PLANE_SHAPE_TYPE]: PlaneShapeType,
  [EnumType.FILE_SOURCE]: FileSource,
  [EnumType.FILE_RETENTION_MODE]: FileRetentionMode,
  [EnumType.FILE_TYPE]: FileType,
  [EnumType.FILE_FORMAT]: FileFormat,
  [EnumType.LINK_TYPE]: LinkType,
  [EnumType.FOLDER_TYPE]: FolderType,
  [EnumType.DATABASE_TYPE]: DatabaseType,
  [EnumType.MACHINE_TYPE]: MachineType,
  [EnumType.MODEL_DEVELOPER]: ModelDeveloper,
  [EnumType.MODEL_PROVIDER]: ModelProvider,
  [EnumType.ACTION_CARDINALITY]: ActionCardinality,
  [EnumType.CURSOR_STATUS]: CursorStatus,
  [EnumType.DAY_OF_WEEK]: DayOfWeek,
  [EnumType.MONTH]: Month,
  [EnumType.SCHEDULE_FREQUENCY]: ScheduleFrequency,
  [EnumType.TIMER_EVENT_TYPE]: TimerEventType,
  [EnumType.TIMER_TYPE]: TimerType,
  [EnumType.TRIGGER_EVENT_TYPE]: TriggerEventType,
  [EnumType.TRIGGER_TYPE]: TriggerType,
  [EnumType.INTERRUPTION_TYPE]: InterruptionType,
  [EnumType.INTERRUPTION_STATUS]: InterruptionStatus,
  [EnumType.INTERRUPTION_RESPONSE]: InterruptionResponse,
  [EnumType.LOG_LEVEL]: LogLevel,
  [EnumType.RUN_STATUS]: RunStatus,
  [EnumType.RUN_EVENT_TYPE]: RunEventType,
  [EnumType.LAYER_TYPE]: LayerType,
  [EnumType.SCENE_EVENT_TYPE]: SceneEventType,
  [EnumType.VARIANT_TYPE]: VariantType,
  [EnumType.VARIANT_STATE_TYPE]: VariantStateType,
  [EnumType.WINDOW_TYPE]: WindowType,
  [EnumType.NOTIFICATION_STATUS]: NotificationStatus,
  [EnumType.NOTIFICATION_EVENT_TYPE]: NotificationEventType,
  [EnumType.THREAD_STATUS]: ThreadStatus,
  [EnumType.FRIENDSHIP_INVITE_EVENT_TYPE]: FriendshipInviteEventType,
  [EnumType.ORGANIZATION_STATUS]: OrganizationStatus,
  [EnumType.SPACE_STATUS]: SpaceStatus,
  [EnumType.USER_STATUS]: UserStatus,
  [EnumType.COLOR_TYPE]: ColorType,
  [EnumType.COLOR_HUE]: ColorHue,
  [EnumType.COLOR_SHADE]: ColorShade,
  [EnumType.COLOR_INTENT]: ColorIntent,
  [EnumType.BORDER_TYPE]: BorderType,
  [EnumType.TRANSITION_TYPE]: TransitionType,
  [EnumType.SPRING_TYPE]: SpringType,
  [EnumType.EFFECT_TYPE]: EffectType,
  [EnumType.REPEAT_TYPE]: RepeatType,
  [EnumType.TEXT_SPLIT_TYPE]: TextSplitType,
  [EnumType.OFFSCREEN_BEHAVIOR]: OffscreenBehavior,
  [EnumType.GRADIENT_TYPE]: GradientType,
  [EnumType.FILL_TYPE]: FillType,
  [EnumType.FILL_POSITION]: FillPosition,
  [EnumType.FILL_SIZE]: FillSize,
  [EnumType.FONT_TYPE]: FontType,
  [EnumType.FONT_WEIGHT]: FontWeight,
  [EnumType.FONT_SIZE]: FontSize,
  [EnumType.TEXT_ALIGN]: TextAlign,
  [EnumType.TEXT_DECORATION]: TextDecoration,
  [EnumType.TEXT_TRANSFORM]: TextTransform,
  [EnumType.SHADOW_TYPE]: ShadowType,
  [EnumType.SHADOW_POSITION]: ShadowPosition,
};
