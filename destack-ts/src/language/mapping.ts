import type {
  Entitlement,
  EntitlementEvent,
  EntitlementExpiredEvent,
  EntitlementGrantedEvent,
  EntitlementRequestedEvent,
  EntitlementRevokedEvent,
  EntitlementType,
} from "@destack/language/access/entitlement";
import type {
  Invite,
  InviteAcceptedEvent,
  InviteEvent,
  InviteRejectedEvent,
  InviteRescindedEvent,
  InviteSentEvent,
} from "@destack/language/access/invite";
import type {
  Membership,
  MembershipEvent,
  MembershipJoinedEvent,
  MembershipLeftEvent,
} from "@destack/language/access/membership";
import type {
  Role,
  RoleAssignedEvent,
  RoleEvent,
  RoleUnassignedEvent,
} from "@destack/language/access/role";
import type {
  Sanction,
  SanctionEvent,
  SanctionExpiredEvent,
  SanctionGrantedEvent,
  SanctionRequestedEvent,
  SanctionRevokedEvent,
  SanctionType,
} from "@destack/language/access/sanction";
import type { Easing } from "@destack/language/animation/easing";
import type {
  Effect,
  EffectStyle,
  EffectType,
  OffscreenBehavior,
  RepeatType,
  TextSplitType,
} from "@destack/language/animation/effect";
import type {
  SpringType,
  Transition,
  TransitionStyle,
  TransitionType,
} from "@destack/language/animation/transition";
import type { Node, Struct } from "@destack/language/core";
import type {
  CascadeAction,
  ClientType,
  Cloud,
  EdgeDirection,
  EdgeType,
  Encoding,
  EnumType,
  EnvironmentType,
  ModeType,
  NodeType,
  OperatingSystem,
  PlatformType,
  PrimitiveType,
  PropertyType,
  Region,
  RegionArea,
  RegionContinent,
  ResourceStatus,
  RoleType,
  RuntimeLanguage,
  ScalarType,
  StoreDomain,
  StoreKey,
  StoreTier,
  StructType,
  Tenancy,
  ToolType,
  TraitType,
  TypeCardinality,
  UniverseCategory,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type { EditEvent, EditOperation, EditType } from "@destack/language/core/builtin/edit";
import type {
  Entity,
  Materialization,
  Record,
  Resource,
  Tag,
  Tagging,
  Variant,
} from "@destack/language/core/builtin/entity";
import type { CustomEvent, Event, EventStatus, Signal } from "@destack/language/core/builtin/event";
import type {
  ConstraintType,
  IndexType,
  MethodCardinality,
  MethodType,
} from "@destack/language/core/builtin/meta";
import type {
  NodeDefinitionReference,
  NodeDefinitionType,
  NodeReference,
  ObjectDefinitionReference,
  ObjectDefinitionType,
  PropertyReference,
  PropertyReferenceType,
  StructDefinitionReference,
  StructDefinitionType,
} from "@destack/language/core/builtin/relation";
import type {
  IsActor,
  IsCustomizable,
  IsDraggable,
  IsExtensible,
  IsFollowable,
  IsInteractive,
  IsIrreversible,
  IsJoinable,
  IsOrdered,
  IsOwnable,
  IsOwned,
  IsReactable,
  IsRunnable,
  IsScriptable,
  IsSelectable,
  IsSourceable,
  IsStarable,
} from "@destack/language/core/builtin/trait";
import type { Action, ActionDefinition } from "@destack/language/core/common/action";
import type {
  BuiltinDefinition,
  ConstantDefinition,
  EnumDefinition,
  NodeDefinition,
  OptionDefinition,
  PropertyDefinition,
  StructDefinition,
  TraitDefinition,
} from "@destack/language/core/common/definition";
import type { CustomEnum, CustomOption } from "@destack/language/core/common/enum";
import type { Icon, IconType } from "@destack/language/core/common/icon";
import type {
  Constraint,
  ConstraintDefinition,
  Index,
  IndexDefinition,
} from "@destack/language/core/common/integrity";
import type { Method, MethodDefinition } from "@destack/language/core/common/method";
import type {
  Migration,
  MigrationDefinition,
  MigrationOperation,
  MigrationOperationDefinition,
  MigrationType,
} from "@destack/language/core/common/migration";
import type { Permission, PermissionDefinition } from "@destack/language/core/common/permission";
import type { CustomProperty } from "@destack/language/core/common/property";
import type {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  Histogram,
  Join,
  JoinType,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  QueryUpdate,
  QueryUpdateType,
  Select,
  Sort,
  SortMode,
  SortType,
} from "@destack/language/core/common/query";
import type { Space, SpaceStatus } from "@destack/language/core/common/space";
import type { CustomStruct, Datum, DatumMutable } from "@destack/language/core/common/struct";
import type { Text, TextSpan, TextSpanType } from "@destack/language/core/common/text";
import type {
  Branch,
  BranchType,
  Snapshot,
  SnapshotStatus,
  SnapshotType,
} from "@destack/language/core/common/time";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  NumberFormat,
  StringConstraint,
  StringFormat,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { File, FileFormat, FileRetentionMode, FileType } from "@destack/language/data/file";
import type { Environment } from "@destack/language/deployment/environment";
import type { LogEvent, LogLevel } from "@destack/language/deployment/log";
import type {
  Run,
  RunCompletedEvent,
  RunEvent,
  RunFailedEvent,
  RunPauseRequestedEvent,
  RunPausedEvent,
  RunResumeRequestedEvent,
  RunResumedEvent,
  RunStartedEvent,
  RunStatus,
  RunStopRequestedEvent,
} from "@destack/language/deployment/run";
import type { SpanEvent } from "@destack/language/deployment/span";
import type { Arrow2D, ArrowHeadType, ArrowShape2D } from "@destack/language/geometry/arrow";
import type { Ellipse2D, EllipseShape2D } from "@destack/language/geometry/ellipse";
import type { Line2D, LineShape2D } from "@destack/language/geometry/line";
import type { Path2D, PathShape2D } from "@destack/language/geometry/path";
import type { Polygon2D, PolygonShape2D } from "@destack/language/geometry/polygon";
import type { Rectangle2D, RectangleShape2D } from "@destack/language/geometry/rectangle";
import type { Shape, Shape2D } from "@destack/language/geometry/shape";
import type {
  Vector,
  Vector2,
  Vector2i,
  Vector3,
  Vector3i,
  Vector4,
  Vector4i,
  Vectorf,
  Vectori,
} from "@destack/language/geometry/vector";
import type {
  Database,
  DatabaseInfo,
  DatabaseType,
} from "@destack/language/infrastructure/database";
import type { Machine, MachineType } from "@destack/language/infrastructure/machine";
import type { ModelDeveloper, ModelProvider } from "@destack/language/intelligence/model";
import type {
  ClipboardEvent,
  CopyEvent,
  CutEvent,
  PasteEvent,
} from "@destack/language/interaction/clipboard";
import type {
  DragEndEvent,
  DragEnterEvent,
  DragEvent,
  DragLeaveEvent,
  DragOverEvent,
  DragStartEvent,
  DropEvent,
} from "@destack/language/interaction/drag";
import type { FocusEvent, FocusInEvent, FocusOutEvent } from "@destack/language/interaction/focus";
import type { InputEvent } from "@destack/language/interaction/input";
import type {
  KeyDownEvent,
  KeyEvent,
  KeyPressEvent,
  KeyUpEvent,
} from "@destack/language/interaction/key";
import type {
  ClickEvent,
  DoubleClickEvent,
  MouseButton,
  MouseEvent,
  SingleClickEvent,
  TripleClickEvent,
  WheelEvent,
} from "@destack/language/interaction/mouse";
import type {
  PointerDownEvent,
  PointerEnterEvent,
  PointerEvent,
  PointerLeaveEvent,
  PointerLongPressEvent,
  PointerMoveEvent,
  PointerOverEvent,
  PointerUpEvent,
} from "@destack/language/interaction/pointer";
import type {
  Cursor,
  CursorStatus,
  EventCursor,
  ScreenCursor,
} from "@destack/language/logic/cursor";
import type { Route } from "@destack/language/logic/route";
import type {
  DayOfWeek,
  Month,
  Schedule,
  ScheduleFrequency,
} from "@destack/language/logic/schedule";
import type { Script } from "@destack/language/logic/script";
import type { Service } from "@destack/language/logic/service";
import type {
  Timer,
  TimerCancelledEvent,
  TimerCompletedEvent,
  TimerEvent,
  TimerPausedEvent,
  TimerResumedEvent,
  TimerStartedEvent,
  TimerType,
} from "@destack/language/logic/timer";
import type { Trigger, TriggerEvent, TriggerType } from "@destack/language/logic/trigger";
import type {
  CounterMeasurementEvent,
  CounterMetric,
  GaugeMeasurementEvent,
  GaugeMetric,
  HistogramMeasurementEvent,
  HistogramMetric,
  MeasurementEvent,
  Metric,
} from "@destack/language/observability/metric";
import type { Layer, LayerType } from "@destack/language/scene/layer";
import type { Scene, SceneEvent } from "@destack/language/scene/scene";
import type { Stage } from "@destack/language/scene/stage";
import type { Window, WindowType } from "@destack/language/scene/window";
import type {
  Follow,
  FollowAddedEvent,
  FollowEvent,
  FollowRemovedEvent,
} from "@destack/language/social/follow";
import type {
  Notification,
  NotificationDismissedEvent,
  NotificationEvent,
  NotificationExpiredEvent,
  NotificationReadEvent,
  NotificationRescindedEvent,
  NotificationSentEvent,
  NotificationStatus,
} from "@destack/language/social/notification";
import type {
  Reaction,
  ReactionAddedEvent,
  ReactionEvent,
  ReactionRemovedEvent,
} from "@destack/language/social/reaction";
import type {
  Star,
  StarAddedEvent,
  StarEvent,
  StarRemovedEvent,
} from "@destack/language/social/star";
import type { Folder, FolderType } from "@destack/language/space/folder";
import type { Border, BorderStyle, BorderType } from "@destack/language/style/border";
import type {
  Color,
  ColorHue,
  ColorIntent,
  ColorShade,
  ColorStyle,
  ColorType,
} from "@destack/language/style/color";
import type {
  Fill,
  FillPosition,
  FillSize,
  FillStyle,
  FillType,
} from "@destack/language/style/fill";
import type {
  Font,
  FontSize,
  FontStyle,
  FontType,
  FontWeight,
  TextAlign,
  TextDecoration,
  TextTransform,
} from "@destack/language/style/font";
import type {
  Gradient,
  GradientStop,
  GradientStyle,
  GradientType,
} from "@destack/language/style/gradient";
import type { Palette } from "@destack/language/style/palette";
import type {
  Shadow,
  ShadowPosition,
  ShadowStyle,
  ShadowType,
} from "@destack/language/style/shadow";
import type {
  Stroke,
  StrokeCap,
  StrokePath,
  StrokePoint,
  StrokeStyle,
  StrokeType,
} from "@destack/language/style/stroke";
import type { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Client } from "@destack/language/universe/client";
import type { Handle } from "@destack/language/universe/handle";
import type { Organization, OrganizationStatus } from "@destack/language/universe/organization";
import type { Team } from "@destack/language/universe/team";
import type { User, UserStatus } from "@destack/language/universe/user";
import type {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  DimensionType,
  Direction,
  Distribute,
  Grid,
  GridSpan,
  Insets,
  Layout,
  Length,
  LengthUnit,
  Overflow,
  Position,
  PositionType,
} from "@destack/language/view/common";
import type { ContentView } from "@destack/language/view/content";
import type { FrameView } from "@destack/language/view/frame";
import type { InputView } from "@destack/language/view/input";
import type { LabelView } from "@destack/language/view/label";
import type { LayoutView } from "@destack/language/view/layout";
import type { NumberInputView } from "@destack/language/view/number";
import type { SliderInputView } from "@destack/language/view/slider";
import type { SplitView } from "@destack/language/view/split";
import type { TextView } from "@destack/language/view/text";
import type { View, ViewEvent } from "@destack/language/view/view";

export type NodeTypeMapping = {
  [NodeType.NODE]: Node;
  [NodeType.ENTITY]: Entity;
  [NodeType.RECORD]: Record;
  [NodeType.RESOURCE]: Resource;
  [NodeType.VARIANT]: Variant;
  [NodeType.TAG]: Tag;
  [NodeType.TAGGING]: Tagging;
  [NodeType.EVENT]: Event;
  [NodeType.CUSTOM_EVENT]: CustomEvent;
  [NodeType.SIGNAL]: Signal;
  [NodeType.EDIT_EVENT]: EditEvent;
  [NodeType.METHOD]: Method;
  [NodeType.ACTION]: Action;
  [NodeType.CUSTOM_ENUM]: CustomEnum;
  [NodeType.CUSTOM_OPTION]: CustomOption;
  [NodeType.INDEX]: Index;
  [NodeType.CONSTRAINT]: Constraint;
  [NodeType.MIGRATION]: Migration;
  [NodeType.MIGRATION_OPERATION]: MigrationOperation;
  [NodeType.PERMISSION]: Permission;
  [NodeType.CUSTOM_PROPERTY]: CustomProperty;
  [NodeType.SPACE]: Space;
  [NodeType.CUSTOM_STRUCT]: CustomStruct;
  [NodeType.BRANCH]: Branch;
  [NodeType.SNAPSHOT]: Snapshot;
  [NodeType.ENTITLEMENT_EVENT]: EntitlementEvent;
  [NodeType.ENTITLEMENT_REQUESTED_EVENT]: EntitlementRequestedEvent;
  [NodeType.ENTITLEMENT_GRANTED_EVENT]: EntitlementGrantedEvent;
  [NodeType.ENTITLEMENT_REVOKED_EVENT]: EntitlementRevokedEvent;
  [NodeType.ENTITLEMENT_EXPIRED_EVENT]: EntitlementExpiredEvent;
  [NodeType.ENTITLEMENT]: Entitlement;
  [NodeType.INVITE_EVENT]: InviteEvent;
  [NodeType.INVITE_SENT_EVENT]: InviteSentEvent;
  [NodeType.INVITE_RESCINDED_EVENT]: InviteRescindedEvent;
  [NodeType.INVITE_ACCEPTED_EVENT]: InviteAcceptedEvent;
  [NodeType.INVITE_REJECTED_EVENT]: InviteRejectedEvent;
  [NodeType.INVITE]: Invite;
  [NodeType.MEMBERSHIP_EVENT]: MembershipEvent;
  [NodeType.MEMBERSHIP_JOINED_EVENT]: MembershipJoinedEvent;
  [NodeType.MEMBERSHIP_LEFT_EVENT]: MembershipLeftEvent;
  [NodeType.MEMBERSHIP]: Membership;
  [NodeType.ROLE_EVENT]: RoleEvent;
  [NodeType.ROLE_ASSIGNED_EVENT]: RoleAssignedEvent;
  [NodeType.ROLE_UNASSIGNED_EVENT]: RoleUnassignedEvent;
  [NodeType.ROLE]: Role;
  [NodeType.SANCTION_EVENT]: SanctionEvent;
  [NodeType.SANCTION_REQUESTED_EVENT]: SanctionRequestedEvent;
  [NodeType.SANCTION_GRANTED_EVENT]: SanctionGrantedEvent;
  [NodeType.SANCTION_REVOKED_EVENT]: SanctionRevokedEvent;
  [NodeType.SANCTION_EXPIRED_EVENT]: SanctionExpiredEvent;
  [NodeType.SANCTION]: Sanction;
  [NodeType.STYLE]: Style;
  [NodeType.COLOR_STYLE]: ColorStyle;
  [NodeType.BORDER_STYLE]: BorderStyle;
  [NodeType.GRADIENT_STYLE]: GradientStyle;
  [NodeType.FILL_STYLE]: FillStyle;
  [NodeType.FONT_STYLE]: FontStyle;
  [NodeType.PALETTE]: Palette;
  [NodeType.SHADOW_STYLE]: ShadowStyle;
  [NodeType.STROKE_STYLE]: StrokeStyle;
  [NodeType.THEME]: Theme;
  [NodeType.TRANSITION_STYLE]: TransitionStyle;
  [NodeType.EFFECT_STYLE]: EffectStyle;
  [NodeType.FILE]: File;
  [NodeType.ENVIRONMENT]: Environment;
  [NodeType.LOG_EVENT]: LogEvent;
  [NodeType.RUN_EVENT]: RunEvent;
  [NodeType.RUN_STARTED_EVENT]: RunStartedEvent;
  [NodeType.RUN_PAUSE_REQUESTED_EVENT]: RunPauseRequestedEvent;
  [NodeType.RUN_PAUSED_EVENT]: RunPausedEvent;
  [NodeType.RUN_RESUME_REQUESTED_EVENT]: RunResumeRequestedEvent;
  [NodeType.RUN_RESUMED_EVENT]: RunResumedEvent;
  [NodeType.RUN_STOP_REQUESTED_EVENT]: RunStopRequestedEvent;
  [NodeType.RUN_FAILED_EVENT]: RunFailedEvent;
  [NodeType.RUN_COMPLETED_EVENT]: RunCompletedEvent;
  [NodeType.RUN]: Run;
  [NodeType.SPAN_EVENT]: SpanEvent;
  [NodeType.VIEW_EVENT]: ViewEvent;
  [NodeType.VIEW]: View;
  [NodeType.CONTENT_VIEW]: ContentView;
  [NodeType.LAYOUT_VIEW]: LayoutView;
  [NodeType.FRAME_VIEW]: FrameView;
  [NodeType.INPUT_VIEW]: InputView;
  [NodeType.LABEL_VIEW]: LabelView;
  [NodeType.NUMBER_INPUT_VIEW]: NumberInputView;
  [NodeType.SLIDER_INPUT_VIEW]: SliderInputView;
  [NodeType.SPLIT_VIEW]: SplitView;
  [NodeType.TEXT_VIEW]: TextView;
  [NodeType.SHAPE]: Shape;
  [NodeType.SHAPE2D]: Shape2D;
  [NodeType.ARROW_SHAPE2D]: ArrowShape2D;
  [NodeType.ELLIPSE_SHAPE2D]: EllipseShape2D;
  [NodeType.LINE_SHAPE2D]: LineShape2D;
  [NodeType.PATH_SHAPE2D]: PathShape2D;
  [NodeType.POLYGON_SHAPE2D]: PolygonShape2D;
  [NodeType.RECTANGLE_SHAPE2D]: RectangleShape2D;
  [NodeType.DATABASE]: Database;
  [NodeType.MACHINE]: Machine;
  [NodeType.INPUT_EVENT]: InputEvent;
  [NodeType.CLIPBOARD_EVENT]: ClipboardEvent;
  [NodeType.COPY_EVENT]: CopyEvent;
  [NodeType.CUT_EVENT]: CutEvent;
  [NodeType.PASTE_EVENT]: PasteEvent;
  [NodeType.DRAG_EVENT]: DragEvent;
  [NodeType.DRAG_START_EVENT]: DragStartEvent;
  [NodeType.DRAG_END_EVENT]: DragEndEvent;
  [NodeType.DRAG_OVER_EVENT]: DragOverEvent;
  [NodeType.DRAG_ENTER_EVENT]: DragEnterEvent;
  [NodeType.DRAG_LEAVE_EVENT]: DragLeaveEvent;
  [NodeType.DROP_EVENT]: DropEvent;
  [NodeType.FOCUS_EVENT]: FocusEvent;
  [NodeType.FOCUS_IN_EVENT]: FocusInEvent;
  [NodeType.FOCUS_OUT_EVENT]: FocusOutEvent;
  [NodeType.KEY_EVENT]: KeyEvent;
  [NodeType.KEY_DOWN_EVENT]: KeyDownEvent;
  [NodeType.KEY_UP_EVENT]: KeyUpEvent;
  [NodeType.KEY_PRESS_EVENT]: KeyPressEvent;
  [NodeType.POINTER_EVENT]: PointerEvent;
  [NodeType.POINTER_DOWN_EVENT]: PointerDownEvent;
  [NodeType.POINTER_UP_EVENT]: PointerUpEvent;
  [NodeType.POINTER_MOVE_EVENT]: PointerMoveEvent;
  [NodeType.POINTER_ENTER_EVENT]: PointerEnterEvent;
  [NodeType.POINTER_OVER_EVENT]: PointerOverEvent;
  [NodeType.POINTER_LEAVE_EVENT]: PointerLeaveEvent;
  [NodeType.POINTER_LONG_PRESS_EVENT]: PointerLongPressEvent;
  [NodeType.MOUSE_EVENT]: MouseEvent;
  [NodeType.CLICK_EVENT]: ClickEvent;
  [NodeType.SINGLE_CLICK_EVENT]: SingleClickEvent;
  [NodeType.DOUBLE_CLICK_EVENT]: DoubleClickEvent;
  [NodeType.TRIPLE_CLICK_EVENT]: TripleClickEvent;
  [NodeType.WHEEL_EVENT]: WheelEvent;
  [NodeType.CURSOR]: Cursor;
  [NodeType.EVENT_CURSOR]: EventCursor;
  [NodeType.SCREEN_CURSOR]: ScreenCursor;
  [NodeType.ROUTE]: Route;
  [NodeType.SCRIPT]: Script;
  [NodeType.SERVICE]: Service;
  [NodeType.TIMER_EVENT]: TimerEvent;
  [NodeType.TIMER_STARTED_EVENT]: TimerStartedEvent;
  [NodeType.TIMER_PAUSED_EVENT]: TimerPausedEvent;
  [NodeType.TIMER_RESUMED_EVENT]: TimerResumedEvent;
  [NodeType.TIMER_COMPLETED_EVENT]: TimerCompletedEvent;
  [NodeType.TIMER_CANCELLED_EVENT]: TimerCancelledEvent;
  [NodeType.TIMER]: Timer;
  [NodeType.TRIGGER_EVENT]: TriggerEvent;
  [NodeType.TRIGGER]: Trigger;
  [NodeType.METRIC]: Metric;
  [NodeType.MEASUREMENT_EVENT]: MeasurementEvent;
  [NodeType.GAUGE_METRIC]: GaugeMetric;
  [NodeType.GAUGE_MEASUREMENT_EVENT]: GaugeMeasurementEvent;
  [NodeType.COUNTER_METRIC]: CounterMetric;
  [NodeType.COUNTER_MEASUREMENT_EVENT]: CounterMeasurementEvent;
  [NodeType.HISTOGRAM_METRIC]: HistogramMetric;
  [NodeType.HISTOGRAM_MEASUREMENT_EVENT]: HistogramMeasurementEvent;
  [NodeType.LAYER]: Layer;
  [NodeType.SCENE_EVENT]: SceneEvent;
  [NodeType.SCENE]: Scene;
  [NodeType.STAGE]: Stage;
  [NodeType.WINDOW]: Window;
  [NodeType.FOLLOW]: Follow;
  [NodeType.FOLLOW_EVENT]: FollowEvent;
  [NodeType.FOLLOW_ADDED_EVENT]: FollowAddedEvent;
  [NodeType.FOLLOW_REMOVED_EVENT]: FollowRemovedEvent;
  [NodeType.NOTIFICATION_EVENT]: NotificationEvent;
  [NodeType.NOTIFICATION_SENT_EVENT]: NotificationSentEvent;
  [NodeType.NOTIFICATION_RESCINDED_EVENT]: NotificationRescindedEvent;
  [NodeType.NOTIFICATION_READ_EVENT]: NotificationReadEvent;
  [NodeType.NOTIFICATION_DISMISSED_EVENT]: NotificationDismissedEvent;
  [NodeType.NOTIFICATION_EXPIRED_EVENT]: NotificationExpiredEvent;
  [NodeType.NOTIFICATION]: Notification;
  [NodeType.REACTION]: Reaction;
  [NodeType.REACTION_EVENT]: ReactionEvent;
  [NodeType.REACTION_ADDED_EVENT]: ReactionAddedEvent;
  [NodeType.REACTION_REMOVED_EVENT]: ReactionRemovedEvent;
  [NodeType.STAR]: Star;
  [NodeType.STAR_EVENT]: StarEvent;
  [NodeType.STAR_ADDED_EVENT]: StarAddedEvent;
  [NodeType.STAR_REMOVED_EVENT]: StarRemovedEvent;
  [NodeType.FOLDER]: Folder;
  [NodeType.CLIENT]: Client;
  [NodeType.HANDLE]: Handle;
  [NodeType.ORGANIZATION]: Organization;
  [NodeType.TEAM]: Team;
  [NodeType.USER]: User;
};

export type TraitTypeMapping = {
  [TraitType.ORDERED]: IsOrdered;
  [TraitType.OWNABLE]: IsOwnable;
  [TraitType.OWNED]: IsOwned;
  [TraitType.JOINABLE]: IsJoinable;
  [TraitType.ACTOR]: IsActor;
  [TraitType.REACTABLE]: IsReactable;
  [TraitType.STARABLE]: IsStarable;
  [TraitType.FOLLOWABLE]: IsFollowable;
  [TraitType.INTERACTIVE]: IsInteractive;
  [TraitType.DRAGGABLE]: IsDraggable;
  [TraitType.SELECTABLE]: IsSelectable;
  [TraitType.SOURCEABLE]: IsSourceable;
  [TraitType.RUNNABLE]: IsRunnable;
  [TraitType.CUSTOMIZABLE]: IsCustomizable;
  [TraitType.SCRIPTABLE]: IsScriptable;
  [TraitType.EXTENSIBLE]: IsExtensible;
  [TraitType.IRREVERSIBLE]: IsIrreversible;
};

export type StructTypeMapping = {
  [StructType.STRUCT]: Struct;
  [StructType.NODE_DEFINITION_REFERENCE]: NodeDefinitionReference;
  [StructType.OBJECT_DEFINITION_REFERENCE]: ObjectDefinitionReference;
  [StructType.STRUCT_DEFINITION_REFERENCE]: StructDefinitionReference;
  [StructType.PROPERTY_REFERENCE]: PropertyReference;
  [StructType.NODE_REFERENCE]: NodeReference;
  [StructType.BUILTIN_DEFINITION]: BuiltinDefinition;
  [StructType.NODE_DEFINITION]: NodeDefinition;
  [StructType.TRAIT_DEFINITION]: TraitDefinition;
  [StructType.STRUCT_DEFINITION]: StructDefinition;
  [StructType.ENUM_DEFINITION]: EnumDefinition;
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition;
  [StructType.OPTION_DEFINITION]: OptionDefinition;
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition;
  [StructType.METHOD_DEFINITION]: MethodDefinition;
  [StructType.ACTION_DEFINITION]: ActionDefinition;
  [StructType.STRING_CONSTRAINT]: StringConstraint;
  [StructType.NUMBER_CONSTRAINT]: NumberConstraint;
  [StructType.COLLECTION_CONSTRAINT]: CollectionConstraint;
  [StructType.NODE_CONSTRAINT]: NodeConstraint;
  [StructType.TYPE]: Type;
  [StructType.ICON]: Icon;
  [StructType.INDEX_DEFINITION]: IndexDefinition;
  [StructType.CONSTRAINT_DEFINITION]: ConstraintDefinition;
  [StructType.MIGRATION_DEFINITION]: MigrationDefinition;
  [StructType.MIGRATION_OPERATION_DEFINITION]: MigrationOperationDefinition;
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition;
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
  [StructType.DATUM]: Datum;
  [StructType.DATUM_MUTABLE]: DatumMutable;
  [StructType.TEXT_SPAN]: TextSpan;
  [StructType.TEXT]: Text;
  [StructType.COLOR]: Color;
  [StructType.BORDER]: Border;
  [StructType.GRADIENT_STOP]: GradientStop;
  [StructType.GRADIENT]: Gradient;
  [StructType.FILL]: Fill;
  [StructType.FONT]: Font;
  [StructType.SHADOW]: Shadow;
  [StructType.STROKE]: Stroke;
  [StructType.STROKE_CAP]: StrokeCap;
  [StructType.STROKE_POINT]: StrokePoint;
  [StructType.STROKE_PATH]: StrokePath;
  [StructType.TRANSITION]: Transition;
  [StructType.EFFECT]: Effect;
  [StructType.LENGTH]: Length;
  [StructType.POSITION]: Position;
  [StructType.DIMENSION]: Dimension;
  [StructType.INSETS]: Insets;
  [StructType.CORNERS]: Corners;
  [StructType.AXIS2]: Axis2;
  [StructType.AXIS3]: Axis3;
  [StructType.GRID]: Grid;
  [StructType.GRID_SPAN]: GridSpan;
  [StructType.ARROW2D]: Arrow2D;
  [StructType.ELLIPSE2D]: Ellipse2D;
  [StructType.LINE2D]: Line2D;
  [StructType.PATH2D]: Path2D;
  [StructType.POLYGON2D]: Polygon2D;
  [StructType.RECTANGLE2D]: Rectangle2D;
  [StructType.VECTOR]: Vector;
  [StructType.VECTORF]: Vectorf;
  [StructType.VECTORI]: Vectori;
  [StructType.VECTOR2]: Vector2;
  [StructType.VECTOR3]: Vector3;
  [StructType.VECTOR4]: Vector4;
  [StructType.VECTOR2I]: Vector2i;
  [StructType.VECTOR3I]: Vector3i;
  [StructType.VECTOR4I]: Vector4i;
  [StructType.DATABASE_INFO]: DatabaseInfo;
  [StructType.SCHEDULE]: Schedule;
};

export type EnumTypeMapping = {
  [EnumType.ENUM_TYPE]: EnumType;
  [EnumType.STRUCT_TYPE]: StructType;
  [EnumType.TRAIT_TYPE]: TraitType;
  [EnumType.NODE_TYPE]: NodeType;
  [EnumType.UNIVERSE_CATEGORY]: UniverseCategory;
  [EnumType.PROPERTY_TYPE]: PropertyType;
  [EnumType.STORE_KEY]: StoreKey;
  [EnumType.STORE_DOMAIN]: StoreDomain;
  [EnumType.STORE_TIER]: StoreTier;
  [EnumType.RUNTIME_LANGUAGE]: RuntimeLanguage;
  [EnumType.PLATFORM_TYPE]: PlatformType;
  [EnumType.OPERATING_SYSTEM]: OperatingSystem;
  [EnumType.ENVIRONMENT_TYPE]: EnvironmentType;
  [EnumType.MODE_TYPE]: ModeType;
  [EnumType.TOOL_TYPE]: ToolType;
  [EnumType.CLOUD]: Cloud;
  [EnumType.REGION_CONTINENT]: RegionContinent;
  [EnumType.REGION_AREA]: RegionArea;
  [EnumType.REGION]: Region;
  [EnumType.EDGE_TYPE]: EdgeType;
  [EnumType.CASCADE_ACTION]: CascadeAction;
  [EnumType.EDGE_DIRECTION]: EdgeDirection;
  [EnumType.ENCODING]: Encoding;
  [EnumType.PRIMITIVE_TYPE]: PrimitiveType;
  [EnumType.TYPE_CARDINALITY]: TypeCardinality;
  [EnumType.SCALAR_TYPE]: ScalarType;
  [EnumType.VALUE_FACTORY]: ValueFactory;
  [EnumType.ROLE_TYPE]: RoleType;
  [EnumType.RESOURCE_STATUS]: ResourceStatus;
  [EnumType.CLIENT_TYPE]: ClientType;
  [EnumType.TENANCY]: Tenancy;
  [EnumType.CONSTRAINT_TYPE]: ConstraintType;
  [EnumType.INDEX_TYPE]: IndexType;
  [EnumType.METHOD_TYPE]: MethodType;
  [EnumType.METHOD_CARDINALITY]: MethodCardinality;
  [EnumType.MATERIALIZATION]: Materialization;
  [EnumType.NODE_DEFINITION_TYPE]: NodeDefinitionType;
  [EnumType.OBJECT_DEFINITION_TYPE]: ObjectDefinitionType;
  [EnumType.STRUCT_DEFINITION_TYPE]: StructDefinitionType;
  [EnumType.PROPERTY_REFERENCE_TYPE]: PropertyReferenceType;
  [EnumType.EVENT_STATUS]: EventStatus;
  [EnumType.EDIT_TYPE]: EditType;
  [EnumType.EDIT_OPERATION]: EditOperation;
  [EnumType.STRING_FORMAT]: StringFormat;
  [EnumType.NUMBER_FORMAT]: NumberFormat;
  [EnumType.ICON_TYPE]: IconType;
  [EnumType.MIGRATION_TYPE]: MigrationType;
  [EnumType.FUNCTION_TYPE]: FunctionType;
  [EnumType.CONDITIONAL_TYPE]: ConditionalType;
  [EnumType.AGGREGATION_TYPE]: AggregationType;
  [EnumType.EXPRESSION_TYPE]: ExpressionType;
  [EnumType.SORT_TYPE]: SortType;
  [EnumType.SORT_MODE]: SortMode;
  [EnumType.JOIN_TYPE]: JoinType;
  [EnumType.QUERY_TYPE]: QueryType;
  [EnumType.QUERY_UPDATE_TYPE]: QueryUpdateType;
  [EnumType.SPACE_STATUS]: SpaceStatus;
  [EnumType.TEXT_SPAN_TYPE]: TextSpanType;
  [EnumType.BRANCH_TYPE]: BranchType;
  [EnumType.SNAPSHOT_TYPE]: SnapshotType;
  [EnumType.SNAPSHOT_STATUS]: SnapshotStatus;
  [EnumType.ENTITLEMENT_TYPE]: EntitlementType;
  [EnumType.SANCTION_TYPE]: SanctionType;
  [EnumType.EASING]: Easing;
  [EnumType.COLOR_TYPE]: ColorType;
  [EnumType.COLOR_HUE]: ColorHue;
  [EnumType.COLOR_SHADE]: ColorShade;
  [EnumType.COLOR_INTENT]: ColorIntent;
  [EnumType.BORDER_TYPE]: BorderType;
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
  [EnumType.TRANSITION_TYPE]: TransitionType;
  [EnumType.SPRING_TYPE]: SpringType;
  [EnumType.EFFECT_TYPE]: EffectType;
  [EnumType.REPEAT_TYPE]: RepeatType;
  [EnumType.TEXT_SPLIT_TYPE]: TextSplitType;
  [EnumType.OFFSCREEN_BEHAVIOR]: OffscreenBehavior;
  [EnumType.FILE_RETENTION_MODE]: FileRetentionMode;
  [EnumType.FILE_TYPE]: FileType;
  [EnumType.FILE_FORMAT]: FileFormat;
  [EnumType.LOG_LEVEL]: LogLevel;
  [EnumType.RUN_STATUS]: RunStatus;
  [EnumType.LAYOUT]: Layout;
  [EnumType.OVERFLOW]: Overflow;
  [EnumType.DIRECTION]: Direction;
  [EnumType.DISTRIBUTE]: Distribute;
  [EnumType.ALIGN]: Align;
  [EnumType.LENGTH_UNIT]: LengthUnit;
  [EnumType.POSITION_TYPE]: PositionType;
  [EnumType.DIMENSION_TYPE]: DimensionType;
  [EnumType.ARROW_HEAD_TYPE]: ArrowHeadType;
  [EnumType.DATABASE_TYPE]: DatabaseType;
  [EnumType.MACHINE_TYPE]: MachineType;
  [EnumType.MODEL_DEVELOPER]: ModelDeveloper;
  [EnumType.MODEL_PROVIDER]: ModelProvider;
  [EnumType.MOUSE_BUTTON]: MouseButton;
  [EnumType.CURSOR_STATUS]: CursorStatus;
  [EnumType.DAY_OF_WEEK]: DayOfWeek;
  [EnumType.MONTH]: Month;
  [EnumType.SCHEDULE_FREQUENCY]: ScheduleFrequency;
  [EnumType.TIMER_TYPE]: TimerType;
  [EnumType.TRIGGER_TYPE]: TriggerType;
  [EnumType.LAYER_TYPE]: LayerType;
  [EnumType.WINDOW_TYPE]: WindowType;
  [EnumType.NOTIFICATION_STATUS]: NotificationStatus;
  [EnumType.FOLDER_TYPE]: FolderType;
  [EnumType.ORGANIZATION_STATUS]: OrganizationStatus;
  [EnumType.USER_STATUS]: UserStatus;
};
