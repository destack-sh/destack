import type {
  Entitlement,
  EntitlementExpiredEvent,
  EntitlementGrantedEvent,
  EntitlementRequestedEvent,
  EntitlementRevokedEvent,
  EntitlementType,
  Invite,
  InviteAcceptedEvent,
  InviteRejectedEvent,
  InviteRescindedEvent,
  InviteSentEvent,
  Membership,
  MembershipJoinedEvent,
  MembershipLeftEvent,
  MembershipPermission,
  Permission,
  PermissionType,
  Role,
  RoleAssignedEvent,
  RoleUnassignedEvent,
  Sanction,
  SanctionExpiredEvent,
  SanctionGrantedEvent,
  SanctionRequestedEvent,
  SanctionRevokedEvent,
  SanctionType,
} from "@destack/language/access";
import type {
  AnnotationShape,
  ArrowHeadType,
  ArrowShape,
  Canvas,
  CanvasType,
  IsShape,
  Line,
  LineShape,
  Polygon,
  PolygonShape,
  PolygonShapeType,
} from "@destack/language/canvas";
import type {
  CascadeAction,
  ClientType,
  Cloud,
  DefaultFactory,
  EdgeDirection,
  EdgeType,
  Entity,
  EnumType,
  EnvironmentType,
  Event,
  Global,
  HasIcon,
  HasName,
  HasSlug,
  IsActionable,
  IsArchivable,
  IsCustomNode,
  IsCustomNodeDefinition,
  IsDeletable,
  IsExtensible,
  IsFollowable,
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
  JoinablePermission,
  LikeFollow,
  LikeInvite,
  LikeMembership,
  LikeTag,
  MaterializationType,
  Measurement,
  Metric,
  ModeType,
  NodePermission,
  NodeType,
  OperatingSystem,
  PlatformType,
  PrimitiveType,
  Region,
  RegionArea,
  RegionContinent,
  Resource,
  ResourceStatus,
  RoleType,
  RuntimeType,
  ScalarType,
  Spatial,
  StoreImplementation,
  StoreType,
  StoreZone,
  StructType,
  Tenancy,
  ToolType,
  TraitType,
  TypeCardinality,
} from "@destack/language/core/builtin";
import type {
  Aggregation,
  AggregationType,
  Align,
  Axis2,
  Axis3,
  Branch,
  Change,
  ChangeDebounce,
  ChangeResult,
  ChangeStatus,
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
  Dimension,
  DimensionType,
  Direction,
  Distribute,
  Edit,
  EditEvent,
  EditOperation,
  EditType,
  EnumDefinition,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  GaugeMeasurement,
  GaugeMetric,
  Grid,
  GridSpan,
  Histogram,
  HistogramMeasurement,
  HistogramMetric,
  Icon,
  IconType,
  Insets,
  Join,
  JoinType,
  Layout,
  Length,
  LengthUnit,
  NodeConstraint,
  NodeDefinition,
  NodeReference,
  NumberConstraint,
  NumberFormat,
  ObjectReference,
  ObjectType,
  OptionDefinition,
  Overflow,
  PermissionDefinition,
  Position,
  PositionType,
  PropertyDefinition,
  PropertyReference,
  PropertyReferenceType,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  QueryUpdate,
  QueryUpdateType,
  RelationReference,
  RelationType,
  Scope,
  Select,
  Selection,
  Snapshot,
  Sort,
  SortMode,
  SortType,
  StringConstraint,
  StringFormat,
  StructDefinition,
  Text,
  TextSpan,
  TextSpanType,
  TraitDefinition,
  Type,
  Value,
  Vector2,
  Vector2i,
  Vector3,
  Vector3i,
  Vector4,
  Vector4i,
} from "@destack/language/core/common";
import type {
  File,
  FileFormat,
  FileRetentionMode,
  FileSource,
  FileType,
  Link,
  LinkType,
} from "@destack/language/data";
import type { Environment } from "@destack/language/deployment";
import type { Folder, FolderType, Tag, Tagging } from "@destack/language/folder";
import type {
  Database,
  DatabaseInfo,
  DatabaseType,
  GalaxyInfo,
  Machine,
  MachineType,
} from "@destack/language/infra";
import type { ModelDeveloper, ModelProvider } from "@destack/language/intelligence";
import type {
  ClickEvent,
  ClipboardEvent,
  CopyEvent,
  CutEvent,
  DoubleClickEvent,
  DragEndEvent,
  DragEnterEvent,
  DragEvent,
  DragLeaveEvent,
  DragOverEvent,
  DragStartEvent,
  DropEvent,
  FocusEvent,
  FocusInEvent,
  FocusOutEvent,
  InputEvent,
  KeyDownEvent,
  KeyPressEvent,
  KeyUpEvent,
  KeyboardEvent,
  LeftClickEvent,
  LongPressEvent,
  MiddleClickEvent,
  MouseButton,
  MouseEvent,
  PasteEvent,
  PointerDownEvent,
  PointerEnterEvent,
  PointerEvent,
  PointerLeaveEvent,
  PointerMoveEvent,
  PointerOverEvent,
  PointerUpEvent,
  RightClickEvent,
  WheelEvent,
} from "@destack/language/interaction";
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
  TimerStartedEvent,
  TimerStoppedEvent,
  TimerType,
  Trigger,
  TriggerStartedEvent,
  TriggerStoppedEvent,
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
  RunCompletedEvent,
  RunFailedEvent,
  RunPauseRequestedEvent,
  RunPausedEvent,
  RunResumeRequestedEvent,
  RunResumedEvent,
  RunStartedEvent,
  RunStatus,
  RunStopRequestedEvent,
  Span,
} from "@destack/language/runtime";
import type {
  Layer,
  LayerType,
  Scene,
  SceneEnteredEvent,
  SceneExitedEvent,
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
  NotificationDismissedEvent,
  NotificationExpiredEvent,
  NotificationReadEvent,
  NotificationRescindedEvent,
  NotificationSentEvent,
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
  FriendshipInviteAcceptedEvent,
  FriendshipInviteRejectedEvent,
  FriendshipInviteRescindedEvent,
  FriendshipInviteSentEvent,
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
  Easing,
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
  Stroke,
  StrokeCap,
  StrokePath,
  StrokePoint,
  StrokeStyle,
  StrokeType,
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
import type { View } from "@destack/language/view";
import type {
  ContainerView,
  CustomView,
  CustomViewDefinition,
  FrameView,
  LabelView,
  SplitView,
} from "@destack/language/view/container";
import type { ContentView, TextView } from "@destack/language/view/content";
import type { InputView, NumberInputView, SliderInputView } from "@destack/language/view/input";
import type { InternalView, WizardView } from "@destack/language/view/internal";
import type { NodeView, ThreadView } from "@destack/language/view/node";

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
  [NodeType.ENTITLEMENT_REQUESTED_EVENT]: EntitlementRequestedEvent;
  [NodeType.ENTITLEMENT_GRANTED_EVENT]: EntitlementGrantedEvent;
  [NodeType.ENTITLEMENT_REVOKED_EVENT]: EntitlementRevokedEvent;
  [NodeType.ENTITLEMENT_EXPIRED_EVENT]: EntitlementExpiredEvent;
  [NodeType.ENTITLEMENT]: Entitlement;
  [NodeType.INVITE_SENT_EVENT]: InviteSentEvent;
  [NodeType.INVITE_RESCINDED_EVENT]: InviteRescindedEvent;
  [NodeType.INVITE_ACCEPTED_EVENT]: InviteAcceptedEvent;
  [NodeType.INVITE_REJECTED_EVENT]: InviteRejectedEvent;
  [NodeType.INVITE]: Invite;
  [NodeType.MEMBERSHIP_JOINED_EVENT]: MembershipJoinedEvent;
  [NodeType.MEMBERSHIP_LEFT_EVENT]: MembershipLeftEvent;
  [NodeType.MEMBERSHIP]: Membership;
  [NodeType.PERMISSION]: Permission;
  [NodeType.ROLE_ASSIGNED_EVENT]: RoleAssignedEvent;
  [NodeType.ROLE_UNASSIGNED_EVENT]: RoleUnassignedEvent;
  [NodeType.ROLE]: Role;
  [NodeType.SANCTION_REQUESTED_EVENT]: SanctionRequestedEvent;
  [NodeType.SANCTION_GRANTED_EVENT]: SanctionGrantedEvent;
  [NodeType.SANCTION_REVOKED_EVENT]: SanctionRevokedEvent;
  [NodeType.SANCTION_EXPIRED_EVENT]: SanctionExpiredEvent;
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
  [NodeType.POLYGON_SHAPE]: PolygonShape;
  [NodeType.FILE]: File;
  [NodeType.LINK]: Link;
  [NodeType.ENVIRONMENT]: Environment;
  [NodeType.FOLDER]: Folder;
  [NodeType.TAG]: Tag;
  [NodeType.TAGGING]: Tagging;
  [NodeType.DATABASE]: Database;
  [NodeType.MACHINE]: Machine;
  [NodeType.POINTER_DOWN_EVENT]: PointerDownEvent;
  [NodeType.POINTER_UP_EVENT]: PointerUpEvent;
  [NodeType.POINTER_MOVE_EVENT]: PointerMoveEvent;
  [NodeType.POINTER_ENTER_EVENT]: PointerEnterEvent;
  [NodeType.POINTER_OVER_EVENT]: PointerOverEvent;
  [NodeType.POINTER_LEAVE_EVENT]: PointerLeaveEvent;
  [NodeType.LONG_PRESS_EVENT]: LongPressEvent;
  [NodeType.LEFT_CLICK_EVENT]: LeftClickEvent;
  [NodeType.RIGHT_CLICK_EVENT]: RightClickEvent;
  [NodeType.MIDDLE_CLICK_EVENT]: MiddleClickEvent;
  [NodeType.DOUBLE_CLICK_EVENT]: DoubleClickEvent;
  [NodeType.WHEEL_EVENT]: WheelEvent;
  [NodeType.KEY_DOWN_EVENT]: KeyDownEvent;
  [NodeType.KEY_UP_EVENT]: KeyUpEvent;
  [NodeType.KEY_PRESS_EVENT]: KeyPressEvent;
  [NodeType.DRAG_START_EVENT]: DragStartEvent;
  [NodeType.DRAG_END_EVENT]: DragEndEvent;
  [NodeType.DRAG_OVER_EVENT]: DragOverEvent;
  [NodeType.DRAG_ENTER_EVENT]: DragEnterEvent;
  [NodeType.DRAG_LEAVE_EVENT]: DragLeaveEvent;
  [NodeType.DROP_EVENT]: DropEvent;
  [NodeType.COPY_EVENT]: CopyEvent;
  [NodeType.CUT_EVENT]: CutEvent;
  [NodeType.PASTE_EVENT]: PasteEvent;
  [NodeType.FOCUS_IN_EVENT]: FocusInEvent;
  [NodeType.FOCUS_OUT_EVENT]: FocusOutEvent;
  [NodeType.ACTION]: Action;
  [NodeType.EVENT_CURSOR]: EventCursor;
  [NodeType.SCREEN_CURSOR]: ScreenCursor;
  [NodeType.THREAD_CURSOR]: ThreadCursor;
  [NodeType.ROUTE]: Route;
  [NodeType.SCRIPT]: Script;
  [NodeType.SERVICE]: Service;
  [NodeType.TIMER_STARTED_EVENT]: TimerStartedEvent;
  [NodeType.TIMER_STOPPED_EVENT]: TimerStoppedEvent;
  [NodeType.TIMER]: Timer;
  [NodeType.TRIGGER_STARTED_EVENT]: TriggerStartedEvent;
  [NodeType.TRIGGER_STOPPED_EVENT]: TriggerStoppedEvent;
  [NodeType.TRIGGER]: Trigger;
  [NodeType.INTERRUPTION]: Interruption;
  [NodeType.LOG]: Log;
  [NodeType.RUN_STARTED_EVENT]: RunStartedEvent;
  [NodeType.RUN_PAUSE_REQUESTED_EVENT]: RunPauseRequestedEvent;
  [NodeType.RUN_PAUSED_EVENT]: RunPausedEvent;
  [NodeType.RUN_RESUME_REQUESTED_EVENT]: RunResumeRequestedEvent;
  [NodeType.RUN_RESUMED_EVENT]: RunResumedEvent;
  [NodeType.RUN_STOP_REQUESTED_EVENT]: RunStopRequestedEvent;
  [NodeType.RUN_FAILED_EVENT]: RunFailedEvent;
  [NodeType.RUN_COMPLETED_EVENT]: RunCompletedEvent;
  [NodeType.RUN]: Run;
  [NodeType.SPAN]: Span;
  [NodeType.LAYER]: Layer;
  [NodeType.SCENE_ENTERED_EVENT]: SceneEnteredEvent;
  [NodeType.SCENE_EXITED_EVENT]: SceneExitedEvent;
  [NodeType.SCENE]: Scene;
  [NodeType.VARIANT]: Variant;
  [NodeType.WINDOW]: Window;
  [NodeType.FOLLOW]: Follow;
  [NodeType.MESSAGE]: Message;
  [NodeType.NOTIFICATION_SENT_EVENT]: NotificationSentEvent;
  [NodeType.NOTIFICATION_RESCINDED_EVENT]: NotificationRescindedEvent;
  [NodeType.NOTIFICATION_READ_EVENT]: NotificationReadEvent;
  [NodeType.NOTIFICATION_DISMISSED_EVENT]: NotificationDismissedEvent;
  [NodeType.NOTIFICATION_EXPIRED_EVENT]: NotificationExpiredEvent;
  [NodeType.NOTIFICATION]: Notification;
  [NodeType.REACTION]: Reaction;
  [NodeType.STAR]: Star;
  [NodeType.THREAD]: Thread;
  [NodeType.AGENT]: Agent;
  [NodeType.CLIENT]: Client;
  [NodeType.FRIENDSHIP]: Friendship;
  [NodeType.FRIENDSHIP_INVITE_SENT_EVENT]: FriendshipInviteSentEvent;
  [NodeType.FRIENDSHIP_INVITE_RESCINDED_EVENT]: FriendshipInviteRescindedEvent;
  [NodeType.FRIENDSHIP_INVITE_ACCEPTED_EVENT]: FriendshipInviteAcceptedEvent;
  [NodeType.FRIENDSHIP_INVITE_REJECTED_EVENT]: FriendshipInviteRejectedEvent;
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
  [NodeType.STROKE_STYLE]: StrokeStyle;
  [NodeType.THEME]: Theme;
};

export type TraitTypeMapping = {
  [TraitType.HAS_NAME]: HasName;
  [TraitType.HAS_SLUG]: HasSlug;
  [TraitType.HAS_ICON]: HasIcon;
  [TraitType.TRACKED]: IsTracked;
  [TraitType.VISUAL]: IsVisual;
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
  [TraitType.EVENT]: Event;
  [TraitType.RESOURCE]: Resource;
  [TraitType.METRIC]: Metric;
  [TraitType.MEASUREMENT]: Measurement;
  [TraitType.VIEW]: View;
  [TraitType.CONTAINER_VIEW]: ContainerView;
  [TraitType.CONTENT_VIEW]: ContentView;
  [TraitType.INPUT_VIEW]: InputView;
  [TraitType.INTERNAL_VIEW]: InternalView;
  [TraitType.NODE_VIEW]: NodeView;
  [TraitType.SHAPE]: IsShape;
  [TraitType.INPUT_EVENT]: InputEvent;
  [TraitType.POINTER_EVENT]: PointerEvent;
  [TraitType.MOUSE_EVENT]: MouseEvent;
  [TraitType.CLICK_EVENT]: ClickEvent;
  [TraitType.KEYBOARD_EVENT]: KeyboardEvent;
  [TraitType.DRAG_EVENT]: DragEvent;
  [TraitType.CLIPBOARD_EVENT]: ClipboardEvent;
  [TraitType.FOCUS_EVENT]: FocusEvent;
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
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition;
  [StructType.TRAIT_DEFINITION]: TraitDefinition;
  [StructType.NODE_DEFINITION]: NodeDefinition;
  [StructType.STRUCT_DEFINITION]: StructDefinition;
  [StructType.ENUM_DEFINITION]: EnumDefinition;
  [StructType.OPTION_DEFINITION]: OptionDefinition;
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition;
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition;
  [StructType.TEXT_SPAN]: TextSpan;
  [StructType.TEXT]: Text;
  [StructType.VECTOR2]: Vector2;
  [StructType.VECTOR3]: Vector3;
  [StructType.VECTOR4]: Vector4;
  [StructType.VECTOR2I]: Vector2i;
  [StructType.VECTOR3I]: Vector3i;
  [StructType.VECTOR4I]: Vector4i;
  [StructType.LENGTH]: Length;
  [StructType.POSITION]: Position;
  [StructType.DIMENSION]: Dimension;
  [StructType.INSETS]: Insets;
  [StructType.CORNERS]: Corners;
  [StructType.AXIS2]: Axis2;
  [StructType.AXIS3]: Axis3;
  [StructType.GRID]: Grid;
  [StructType.GRID_SPAN]: GridSpan;
  [StructType.LINE]: Line;
  [StructType.POLYGON]: Polygon;
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
  [StructType.STROKE]: Stroke;
  [StructType.STROKE_CAP]: StrokeCap;
  [StructType.STROKE_POINT]: StrokePoint;
  [StructType.STROKE_PATH]: StrokePath;
};

export type EnumTypeMapping = {
  [EnumType.ENUM_TYPE]: EnumType;
  [EnumType.STRUCT_TYPE]: StructType;
  [EnumType.TRAIT_TYPE]: TraitType;
  [EnumType.NODE_TYPE]: NodeType;
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
  [EnumType.ENTITLEMENT_TYPE]: EntitlementType;
  [EnumType.MEMBERSHIP_PERMISSION]: MembershipPermission;
  [EnumType.PERMISSION_TYPE]: PermissionType;
  [EnumType.SANCTION_TYPE]: SanctionType;
  [EnumType.ARROW_HEAD_TYPE]: ArrowHeadType;
  [EnumType.CANVAS_TYPE]: CanvasType;
  [EnumType.POLYGON_SHAPE_TYPE]: PolygonShapeType;
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
  [EnumType.MOUSE_BUTTON]: MouseButton;
  [EnumType.ACTION_CARDINALITY]: ActionCardinality;
  [EnumType.CURSOR_STATUS]: CursorStatus;
  [EnumType.DAY_OF_WEEK]: DayOfWeek;
  [EnumType.MONTH]: Month;
  [EnumType.SCHEDULE_FREQUENCY]: ScheduleFrequency;
  [EnumType.TIMER_TYPE]: TimerType;
  [EnumType.TRIGGER_TYPE]: TriggerType;
  [EnumType.INTERRUPTION_TYPE]: InterruptionType;
  [EnumType.INTERRUPTION_STATUS]: InterruptionStatus;
  [EnumType.INTERRUPTION_RESPONSE]: InterruptionResponse;
  [EnumType.LOG_LEVEL]: LogLevel;
  [EnumType.RUN_STATUS]: RunStatus;
  [EnumType.LAYER_TYPE]: LayerType;
  [EnumType.VARIANT_TYPE]: VariantType;
  [EnumType.VARIANT_STATE_TYPE]: VariantStateType;
  [EnumType.WINDOW_TYPE]: WindowType;
  [EnumType.NOTIFICATION_STATUS]: NotificationStatus;
  [EnumType.THREAD_STATUS]: ThreadStatus;
  [EnumType.ORGANIZATION_STATUS]: OrganizationStatus;
  [EnumType.SPACE_STATUS]: SpaceStatus;
  [EnumType.USER_STATUS]: UserStatus;
  [EnumType.COLOR_TYPE]: ColorType;
  [EnumType.COLOR_HUE]: ColorHue;
  [EnumType.COLOR_SHADE]: ColorShade;
  [EnumType.COLOR_INTENT]: ColorIntent;
  [EnumType.BORDER_TYPE]: BorderType;
  [EnumType.EASING]: Easing;
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
  [EnumType.STROKE_TYPE]: StrokeType;
};
