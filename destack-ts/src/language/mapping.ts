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
import type { Permission, PermissionType } from "@destack/language/access/permission";
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
import type { AnnotationShape } from "@destack/language/canvas/annotation";
import type { Arrow, ArrowHeadType, ArrowShape } from "@destack/language/canvas/arrow";
import type { Canvas, CanvasType } from "@destack/language/canvas/canvas";
import type { Line, LineShape } from "@destack/language/canvas/line";
import type { Shape } from "@destack/language/canvas/shape";
import type { Node, Struct } from "@destack/language/core";
import type {
  CascadeAction,
  ClientType,
  Cloud,
  EdgeDirection,
  EdgeType,
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
  StoreImplementation,
  StoreType,
  StructType,
  Tenancy,
  ToolType,
  TraitType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type {
  CustomEntityDefinition,
  CustomTraitDefinition,
  Entity,
  Materialization,
  Metric,
  Record,
  Resource,
  Snapshot,
  SnapshotStatus,
  SnapshotType,
} from "@destack/language/core/builtin/entity";
import type {
  ChangeEvent,
  CustomEventDefinition,
  EditEvent,
  Event,
  MeasurementEvent,
  QueryEvent,
  Signal,
} from "@destack/language/core/builtin/event";
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
  IsArchivable,
  IsCustomizable,
  IsDeletable,
  IsExtensible,
  IsFollowable,
  IsGlobal,
  IsIrreversible,
  IsJoinable,
  IsOrdered,
  IsOwnable,
  IsOwner,
  IsReactable,
  IsRunnable,
  IsScriptable,
  IsSourceable,
  IsSpatial,
  IsStarable,
  IsSubject,
  IsTaggable,
} from "@destack/language/core/builtin/trait";
import type {
  ActionDefinition,
  BuiltinDefinition,
  BuiltinObjectDefinition,
  ConstantDefinition,
  EnumDefinition,
  MethodDefinition,
  NodeDefinition,
  OptionDefinition,
  OptionGroupDefinition,
  PermissionDefinition,
  PropertyDefinition,
  PropertyGroupDefinition,
  StructDefinition,
  TraitDefinition,
} from "@destack/language/core/common/definition";
import type {
  Change,
  ChangeDebounce,
  ChangeResult,
  ChangeStatus,
  Edit,
  EditOperation,
  EditType,
  Origin,
} from "@destack/language/core/common/edit";
import type {
  CustomEnumDefinition,
  CustomOption,
  CustomOptionGroup,
} from "@destack/language/core/common/enum";
import type { Icon, IconType } from "@destack/language/core/common/icon";
import type {
  CounterMeasurementEvent,
  CounterMetric,
  GaugeMeasurementEvent,
  GaugeMetric,
  HistogramMeasurementEvent,
  HistogramMetric,
} from "@destack/language/core/common/metric";
import type { CustomProperty, CustomPropertyGroup } from "@destack/language/core/common/property";
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
  Selection,
  Sort,
  SortMode,
  SortType,
} from "@destack/language/core/common/query";
import type { CustomStruct, CustomStructDefinition } from "@destack/language/core/common/struct";
import type { Text, TextSpan, TextSpanType } from "@destack/language/core/common/text";
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
import type {
  Vector,
  Vector2f,
  Vector2i,
  Vector3f,
  Vector3i,
  Vector4f,
  Vector4i,
  Vectorf,
  Vectori,
} from "@destack/language/core/common/vector";
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
} from "@destack/language/core/common/view";
import type {
  File,
  FileFormat,
  FileRetentionMode,
  FileSource,
  FileType,
} from "@destack/language/data/file";
import type { Environment } from "@destack/language/deployment/environment";
import type { Folder, FolderType } from "@destack/language/folder/folder";
import type { Tag, Tagging } from "@destack/language/folder/tag";
import type { Database, DatabaseInfo, DatabaseType } from "@destack/language/infra/database";
import type { GalaxyInfo } from "@destack/language/infra/galaxy";
import type { Machine, MachineType } from "@destack/language/infra/machine";
import type { ModelDeveloper, ModelProvider } from "@destack/language/intelligence/model";
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
} from "@destack/language/interaction/input";
import type { Action, ActionCardinality } from "@destack/language/logic/action";
import type {
  Cursor,
  CursorStatus,
  EventCursor,
  ScreenCursor,
  ThreadCursor,
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
  TimerStartedEvent,
  TimerType,
} from "@destack/language/logic/timer";
import type { Trigger, TriggerEvent, TriggerType } from "@destack/language/logic/trigger";
import type {
  Interruption,
  InterruptionResponse,
  InterruptionStatus,
  InterruptionType,
} from "@destack/language/runtime/interruption";
import type { LogEvent, LogLevel } from "@destack/language/runtime/log";
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
} from "@destack/language/runtime/run";
import type { SpanEvent } from "@destack/language/runtime/span";
import type { Layer, LayerType } from "@destack/language/scene/layer";
import type {
  Scene,
  SceneEnteredEvent,
  SceneEvent,
  SceneExitedEvent,
} from "@destack/language/scene/scene";
import type { Variant, VariantStateType, VariantType } from "@destack/language/scene/variant";
import type { Window, WindowType } from "@destack/language/scene/window";
import type { Follow } from "@destack/language/social/follow";
import type { Message } from "@destack/language/social/message";
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
import type { Reaction } from "@destack/language/social/reaction";
import type { Star } from "@destack/language/social/star";
import type { Thread, ThreadStatus } from "@destack/language/social/thread";
import type { Agent } from "@destack/language/space/agent";
import type { Client } from "@destack/language/space/client";
import type {
  Friendship,
  FriendshipInvite,
  FriendshipInviteAcceptedEvent,
  FriendshipInviteEvent,
  FriendshipInviteRejectedEvent,
  FriendshipInviteRescindedEvent,
  FriendshipInviteSentEvent,
} from "@destack/language/space/friendship";
import type { Handle } from "@destack/language/space/handle";
import type { Organization, OrganizationStatus } from "@destack/language/space/organization";
import type { Space, SpaceStatus } from "@destack/language/space/space";
import type { Team } from "@destack/language/space/team";
import type { Universe } from "@destack/language/space/universe";
import type { User, UserStatus } from "@destack/language/space/user";
import type { Branch } from "@destack/language/spacetime/branch";
import type { Border, BorderStyle, BorderType } from "@destack/language/style/border";
import type {
  Color,
  ColorHue,
  ColorIntent,
  ColorShade,
  ColorStyle,
  ColorType,
} from "@destack/language/style/color";
import type { Easing } from "@destack/language/style/easing";
import type {
  Effect,
  EffectStyle,
  EffectType,
  OffscreenBehavior,
  RepeatType,
  TextSplitType,
} from "@destack/language/style/effect";
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
import type {
  SpringType,
  Transition,
  TransitionStyle,
  TransitionType,
} from "@destack/language/style/transition";
import type { ContainerView } from "@destack/language/view/container/container";
import type { FrameView } from "@destack/language/view/container/frame";
import type { LabelView } from "@destack/language/view/container/label";
import type { SplitView } from "@destack/language/view/container/split";
import type { ContentView } from "@destack/language/view/content/content";
import type { TextView } from "@destack/language/view/content/text";
import type { InputView } from "@destack/language/view/input/input";
import type { NumberInputView } from "@destack/language/view/input/number";
import type { SliderInputView } from "@destack/language/view/input/slider";
import type { InternalView } from "@destack/language/view/internal/internal";
import type { View } from "@destack/language/view/view";

export type NodeTypeMapping = {
  [NodeType.NODE]: Node;
  [NodeType.ENTITY]: Entity;
  [NodeType.CUSTOM_ENTITY_DEFINITION]: CustomEntityDefinition;
  [NodeType.CUSTOM_TRAIT_DEFINITION]: CustomTraitDefinition;
  [NodeType.RECORD]: Record;
  [NodeType.RESOURCE]: Resource;
  [NodeType.METRIC]: Metric;
  [NodeType.SNAPSHOT]: Snapshot;
  [NodeType.EVENT]: Event;
  [NodeType.CUSTOM_EVENT_DEFINITION]: CustomEventDefinition;
  [NodeType.SIGNAL]: Signal;
  [NodeType.EDIT_EVENT]: EditEvent;
  [NodeType.CHANGE_EVENT]: ChangeEvent;
  [NodeType.QUERY_EVENT]: QueryEvent;
  [NodeType.MEASUREMENT_EVENT]: MeasurementEvent;
  [NodeType.CUSTOM_ENUM_DEFINITION]: CustomEnumDefinition;
  [NodeType.CUSTOM_OPTION]: CustomOption;
  [NodeType.CUSTOM_OPTION_GROUP]: CustomOptionGroup;
  [NodeType.GAUGE_METRIC]: GaugeMetric;
  [NodeType.GAUGE_MEASUREMENT_EVENT]: GaugeMeasurementEvent;
  [NodeType.COUNTER_METRIC]: CounterMetric;
  [NodeType.COUNTER_MEASUREMENT_EVENT]: CounterMeasurementEvent;
  [NodeType.HISTOGRAM_METRIC]: HistogramMetric;
  [NodeType.HISTOGRAM_MEASUREMENT_EVENT]: HistogramMeasurementEvent;
  [NodeType.CUSTOM_PROPERTY]: CustomProperty;
  [NodeType.CUSTOM_PROPERTY_GROUP]: CustomPropertyGroup;
  [NodeType.CUSTOM_STRUCT_DEFINITION]: CustomStructDefinition;
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
  [NodeType.PERMISSION]: Permission;
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
  [NodeType.VIEW]: View;
  [NodeType.CONTAINER_VIEW]: ContainerView;
  [NodeType.FRAME_VIEW]: FrameView;
  [NodeType.LABEL_VIEW]: LabelView;
  [NodeType.SPLIT_VIEW]: SplitView;
  [NodeType.CONTENT_VIEW]: ContentView;
  [NodeType.TEXT_VIEW]: TextView;
  [NodeType.INPUT_VIEW]: InputView;
  [NodeType.NUMBER_INPUT_VIEW]: NumberInputView;
  [NodeType.SLIDER_INPUT_VIEW]: SliderInputView;
  [NodeType.INTERNAL_VIEW]: InternalView;
  [NodeType.SHAPE]: Shape;
  [NodeType.ANNOTATION_SHAPE]: AnnotationShape;
  [NodeType.ARROW_SHAPE]: ArrowShape;
  [NodeType.CANVAS]: Canvas;
  [NodeType.LINE_SHAPE]: LineShape;
  [NodeType.FILE]: File;
  [NodeType.ENVIRONMENT]: Environment;
  [NodeType.FOLDER]: Folder;
  [NodeType.TAG]: Tag;
  [NodeType.TAGGING]: Tagging;
  [NodeType.DATABASE]: Database;
  [NodeType.MACHINE]: Machine;
  [NodeType.INPUT_EVENT]: InputEvent;
  [NodeType.POINTER_EVENT]: PointerEvent;
  [NodeType.POINTER_DOWN_EVENT]: PointerDownEvent;
  [NodeType.POINTER_UP_EVENT]: PointerUpEvent;
  [NodeType.POINTER_MOVE_EVENT]: PointerMoveEvent;
  [NodeType.POINTER_ENTER_EVENT]: PointerEnterEvent;
  [NodeType.POINTER_OVER_EVENT]: PointerOverEvent;
  [NodeType.POINTER_LEAVE_EVENT]: PointerLeaveEvent;
  [NodeType.LONG_PRESS_EVENT]: LongPressEvent;
  [NodeType.MOUSE_EVENT]: MouseEvent;
  [NodeType.CLICK_EVENT]: ClickEvent;
  [NodeType.LEFT_CLICK_EVENT]: LeftClickEvent;
  [NodeType.RIGHT_CLICK_EVENT]: RightClickEvent;
  [NodeType.MIDDLE_CLICK_EVENT]: MiddleClickEvent;
  [NodeType.DOUBLE_CLICK_EVENT]: DoubleClickEvent;
  [NodeType.WHEEL_EVENT]: WheelEvent;
  [NodeType.KEYBOARD_EVENT]: KeyboardEvent;
  [NodeType.KEY_DOWN_EVENT]: KeyDownEvent;
  [NodeType.KEY_UP_EVENT]: KeyUpEvent;
  [NodeType.KEY_PRESS_EVENT]: KeyPressEvent;
  [NodeType.DRAG_EVENT]: DragEvent;
  [NodeType.DRAG_START_EVENT]: DragStartEvent;
  [NodeType.DRAG_END_EVENT]: DragEndEvent;
  [NodeType.DRAG_OVER_EVENT]: DragOverEvent;
  [NodeType.DRAG_ENTER_EVENT]: DragEnterEvent;
  [NodeType.DRAG_LEAVE_EVENT]: DragLeaveEvent;
  [NodeType.DROP_EVENT]: DropEvent;
  [NodeType.CLIPBOARD_EVENT]: ClipboardEvent;
  [NodeType.COPY_EVENT]: CopyEvent;
  [NodeType.CUT_EVENT]: CutEvent;
  [NodeType.PASTE_EVENT]: PasteEvent;
  [NodeType.FOCUS_EVENT]: FocusEvent;
  [NodeType.FOCUS_IN_EVENT]: FocusInEvent;
  [NodeType.FOCUS_OUT_EVENT]: FocusOutEvent;
  [NodeType.ACTION]: Action;
  [NodeType.CURSOR]: Cursor;
  [NodeType.EVENT_CURSOR]: EventCursor;
  [NodeType.SCREEN_CURSOR]: ScreenCursor;
  [NodeType.THREAD_CURSOR]: ThreadCursor;
  [NodeType.ROUTE]: Route;
  [NodeType.SCRIPT]: Script;
  [NodeType.SERVICE]: Service;
  [NodeType.TIMER_EVENT]: TimerEvent;
  [NodeType.TIMER_STARTED_EVENT]: TimerStartedEvent;
  [NodeType.TIMER_COMPLETED_EVENT]: TimerCompletedEvent;
  [NodeType.TIMER_CANCELLED_EVENT]: TimerCancelledEvent;
  [NodeType.TIMER]: Timer;
  [NodeType.TRIGGER_EVENT]: TriggerEvent;
  [NodeType.TRIGGER]: Trigger;
  [NodeType.INTERRUPTION]: Interruption;
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
  [NodeType.LAYER]: Layer;
  [NodeType.SCENE_EVENT]: SceneEvent;
  [NodeType.SCENE_ENTERED_EVENT]: SceneEnteredEvent;
  [NodeType.SCENE_EXITED_EVENT]: SceneExitedEvent;
  [NodeType.SCENE]: Scene;
  [NodeType.VARIANT]: Variant;
  [NodeType.WINDOW]: Window;
  [NodeType.FOLLOW]: Follow;
  [NodeType.MESSAGE]: Message;
  [NodeType.NOTIFICATION_EVENT]: NotificationEvent;
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
  [NodeType.FRIENDSHIP_INVITE_EVENT]: FriendshipInviteEvent;
  [NodeType.FRIENDSHIP_INVITE_SENT_EVENT]: FriendshipInviteSentEvent;
  [NodeType.FRIENDSHIP_INVITE_RESCINDED_EVENT]: FriendshipInviteRescindedEvent;
  [NodeType.FRIENDSHIP_INVITE_ACCEPTED_EVENT]: FriendshipInviteAcceptedEvent;
  [NodeType.FRIENDSHIP_INVITE_REJECTED_EVENT]: FriendshipInviteRejectedEvent;
  [NodeType.FRIENDSHIP_INVITE]: FriendshipInvite;
  [NodeType.HANDLE]: Handle;
  [NodeType.ORGANIZATION]: Organization;
  [NodeType.SPACE]: Space;
  [NodeType.TEAM]: Team;
  [NodeType.UNIVERSE]: Universe;
  [NodeType.USER]: User;
  [NodeType.BRANCH]: Branch;
  [NodeType.STYLE]: Style;
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
  [TraitType.GLOBAL]: IsGlobal;
  [TraitType.SPATIAL]: IsSpatial;
  [TraitType.ORDERED]: IsOrdered;
  [TraitType.ARCHIVABLE]: IsArchivable;
  [TraitType.DELETABLE]: IsDeletable;
  [TraitType.CUSTOMIZABLE]: IsCustomizable;
  [TraitType.EXTENSIBLE]: IsExtensible;
  [TraitType.IRREVERSIBLE]: IsIrreversible;
  [TraitType.OWNABLE]: IsOwnable;
  [TraitType.JOINABLE]: IsJoinable;
  [TraitType.SUBJECT]: IsSubject;
  [TraitType.OWNER]: IsOwner;
  [TraitType.TAGGABLE]: IsTaggable;
  [TraitType.REACTABLE]: IsReactable;
  [TraitType.STARABLE]: IsStarable;
  [TraitType.FOLLOWABLE]: IsFollowable;
  [TraitType.SOURCEABLE]: IsSourceable;
  [TraitType.SCRIPTABLE]: IsScriptable;
  [TraitType.RUNNABLE]: IsRunnable;
};

export type StructTypeMapping = {
  [StructType.STRUCT]: Struct;
  [StructType.NODE_DEFINITION_REFERENCE]: NodeDefinitionReference;
  [StructType.OBJECT_DEFINITION_REFERENCE]: ObjectDefinitionReference;
  [StructType.STRUCT_DEFINITION_REFERENCE]: StructDefinitionReference;
  [StructType.PROPERTY_REFERENCE]: PropertyReference;
  [StructType.NODE_REFERENCE]: NodeReference;
  [StructType.BUILTIN_DEFINITION]: BuiltinDefinition;
  [StructType.BUILTIN_OBJECT_DEFINITION]: BuiltinObjectDefinition;
  [StructType.NODE_DEFINITION]: NodeDefinition;
  [StructType.TRAIT_DEFINITION]: TraitDefinition;
  [StructType.STRUCT_DEFINITION]: StructDefinition;
  [StructType.ENUM_DEFINITION]: EnumDefinition;
  [StructType.PROPERTY_DEFINITION]: PropertyDefinition;
  [StructType.PROPERTY_GROUP_DEFINITION]: PropertyGroupDefinition;
  [StructType.OPTION_DEFINITION]: OptionDefinition;
  [StructType.OPTION_GROUP_DEFINITION]: OptionGroupDefinition;
  [StructType.CONSTANT_DEFINITION]: ConstantDefinition;
  [StructType.METHOD_DEFINITION]: MethodDefinition;
  [StructType.ACTION_DEFINITION]: ActionDefinition;
  [StructType.PERMISSION_DEFINITION]: PermissionDefinition;
  [StructType.EDIT]: Edit;
  [StructType.ORIGIN]: Origin;
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
  [StructType.CUSTOM_STRUCT]: CustomStruct;
  [StructType.TEXT_SPAN]: TextSpan;
  [StructType.TEXT]: Text;
  [StructType.VECTOR]: Vector;
  [StructType.VECTORF]: Vectorf;
  [StructType.VECTORI]: Vectori;
  [StructType.VECTOR2F]: Vector2f;
  [StructType.VECTOR3F]: Vector3f;
  [StructType.VECTOR4F]: Vector4f;
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
  [StructType.ARROW]: Arrow;
  [StructType.LINE]: Line;
  [StructType.DATABASE_INFO]: DatabaseInfo;
  [StructType.GALAXY_INFO]: GalaxyInfo;
  [StructType.SCHEDULE]: Schedule;
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
  [EnumType.PROPERTY_TYPE]: PropertyType;
  [EnumType.STORE_TYPE]: StoreType;
  [EnumType.STORE_IMPLEMENTATION]: StoreImplementation;
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
  [EnumType.PRIMITIVE_TYPE]: PrimitiveType;
  [EnumType.TYPE_CARDINALITY]: TypeCardinality;
  [EnumType.SCALAR_TYPE]: ScalarType;
  [EnumType.VALUE_FACTORY]: ValueFactory;
  [EnumType.ROLE_TYPE]: RoleType;
  [EnumType.RESOURCE_STATUS]: ResourceStatus;
  [EnumType.CLIENT_TYPE]: ClientType;
  [EnumType.TENANCY]: Tenancy;
  [EnumType.MATERIALIZATION]: Materialization;
  [EnumType.NODE_DEFINITION_TYPE]: NodeDefinitionType;
  [EnumType.OBJECT_DEFINITION_TYPE]: ObjectDefinitionType;
  [EnumType.STRUCT_DEFINITION_TYPE]: StructDefinitionType;
  [EnumType.PROPERTY_REFERENCE_TYPE]: PropertyReferenceType;
  [EnumType.SNAPSHOT_TYPE]: SnapshotType;
  [EnumType.SNAPSHOT_STATUS]: SnapshotStatus;
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
  [EnumType.PERMISSION_TYPE]: PermissionType;
  [EnumType.SANCTION_TYPE]: SanctionType;
  [EnumType.ARROW_HEAD_TYPE]: ArrowHeadType;
  [EnumType.CANVAS_TYPE]: CanvasType;
  [EnumType.FILE_SOURCE]: FileSource;
  [EnumType.FILE_RETENTION_MODE]: FileRetentionMode;
  [EnumType.FILE_TYPE]: FileType;
  [EnumType.FILE_FORMAT]: FileFormat;
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
