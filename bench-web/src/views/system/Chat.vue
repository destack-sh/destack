<script lang="ts" setup>
import { TITLE_CONSTRAINT, toCamelName } from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import { emptyText, isTextEmpty, trimText } from "@/language/text";
import {
  BenchType,
  BoxData,
  IconData,
  MessageData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  TextData,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  toNodeRefOneOf,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { findExistingConnectionOrError, useExistingConnection } from "@/system/connection";
import { canvas, inspectionPtr, pkgGraph as localPkgGraph, pkg } from "@/system/space";
import { user } from "@/system/user";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import { makeSelection } from "@/ui/canvas";
import { DEFAULT_USER_ICON, IconInline, makeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { graphIndex } from "@/ui/search";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { getElement } from "@/utils/element";
import { generateRandomName } from "@/utils/naming";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate, tsToDt } from "@/utils/time";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodePath from "@/views/builtins/NodePath.vue";
import { makeViewId, viewEmits, type FocusAnchor, type ViewComponent, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Text from "@/views/content/Text.vue";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, watch, type Ref } from "vue";

const HEADER_HEIGHT_NORMAL = 36;
const HEADER_HEIGHT_COMPACT = 32;
const MAX_WIDTH = 800;
const MIN_WIDTH = 360;
const DEFAULT_WIDTH = 360;
const DEFAULT_HEIGHT = 480;
const DEFAULT_MAX_INPUT_HEIGHT = 120;
const MIN_INPUT_HEIGHT = 40;
const MIN_GUTTER_WIDTH = 4;
const ASIDE_WIDTH_NORMAL = 36;
const ASIDE_WIDTH_COMPACT = 36;
const HANDLE_WIDTH = 6;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size?: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "title" | "nodePtr" | "focus" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr));

const headerHeight = computed(() => (props.variant == Variant.COMPACT ? HEADER_HEIGHT_COMPACT : HEADER_HEIGHT_NORMAL));
const asideSize = computed(() => (props.variant == Variant.COMPACT ? ASIDE_WIDTH_COMPACT : ASIDE_WIDTH_NORMAL));
const maxInputHeight = computed(() =>
  props.size != null ? (props.size.height - headerHeight.value) / 2 : DEFAULT_MAX_INPUT_HEIGHT,
);
const draftRef: Ref<HTMLDivElement | null> = ref(null);
const inputSize = useElementSize(draftRef);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const messageRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

// NOTE: threadPtr can point to a message node if we already have a thread or to any node to create a thread on
const { graph: spaceGraph } = useExistingConnection(self);
const preparedPkgConnection = useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const selfView = spaceGraph.getRef(self);
const node = pkgGraph.getRef(nodePtr);
const thread = computed(() => (node.value != null && isNode(node.value, NodeType.MESSAGE) ? node.value : null));
const messages = pkgGraph.getChildrenRef(node, NodeType.MESSAGE);
const ancestors = pkgGraph.getAncestorsRef(nodePtr, { includeSelf: false });
const context = computed(() => {
  if (node.value != null && !isNode(node.value, NodeType.MESSAGE)) return node.value;
  else return ancestors.value.find((n) => n.metatype != ObjectType.MESSAGE) ?? pkg.value;
});

// sync view title with context name
watch(
  () => (context.value as any)?.name,
  () => {
    if (context.value != null && selfView.value != null) {
      const targetTitle =
        context.value?.id == pkg.value?.id || !("name" in context.value)
          ? toCamelName(ViewType, ViewType.CHAT)
          : context.value.name;
      if (targetTitle != selfView.value.title) {
        pkgConnection.tx.update(selfView.value, { title: targetTitle }, { debounce: "long" });
      }
    }
  },
  { immediate: true },
);

function getAuthor(message: MessageData): { name: string; icon: IconData } {
  // NOTE :Broken: load correct name/icon for message author
  if (message.createdByPtr?.id == user.value?.id)
    return { name: user.value!.name, icon: user.value!.icon ?? DEFAULT_USER_ICON };
  else return { name: "Bench", icon: makeIcon("fas fa-robot") };
}

type RenderedMessage = {
  message: MessageData;
  author: { name: string; icon: IconData };
  isContinued: boolean;
  isContinuationBreak: boolean;
  replyToMessage: MessageData | null;
  replyToAuthor: { name: string; icon: IconData } | null;
};
const renderedMessages = computed(() => {
  // collapse continued messages if they are from same author within 5 minutes (and not a reply)
  const result: RenderedMessage[] = [];
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const author = getAuthor(message);
    const lastMessage = result[result.length - 1]?.message;
    const isContinued =
      lastMessage != null &&
      lastMessage.createdByPtr?.id == message.createdByPtr?.id &&
      message.createdAt!.seconds - lastMessage.createdAt!.seconds < 300;
    const isContinuationBreak = !isContinued && lastMessage != null;
    // NOTE: replyTo message is technically not reactive in its own
    //  but 1) replies should be to messages in the thread, so that's auto reactive and 2) it's probably fine?
    const replyToMessage = message.replyToPtr != null ? (pkgGraph.get(message.replyToPtr) as MessageData | null) : null;
    const replyToAuthor = replyToMessage != null ? getAuthor(replyToMessage) : null;
    result.push({ message, author, isContinued, isContinuationBreak, replyToMessage, replyToAuthor });
  }
  return result;
});

const stickToEnd = ref(true);
const replyingTo: Ref<MessageData | null> = ref(null);
const text: Ref<TextData> = ref(emptyText());
const canSubmit = computed(() => !isTextEmpty(text.value));

// auto-sticky/unsticky when scrolled to end
watch(
  () => scrollRef.value?.isAtEnd,
  () => {
    stickToEnd.value = scrollRef.value?.isAtEnd ?? true;
  },
);

function followEnd() {
  stickToEnd.value = true;
  scrollRef.value?.scrollToEnd();
}

function replyTo(message: MessageData) {
  replyingTo.value = message;
  textRef.value?.focus!("center");
}

function createNewThread(parent: AnyNodeData, title: string = generateRandomName()) {
  const packagePtr = isNode(parent, NodeType.PACKAGE) ? toPlainNodeRef(parent) : (parent as any).packagePtr;
  if (packagePtr == null) throw new Error(`parent is not in a package: ${describeNode(parent)}`);
  const tx = findExistingConnectionOrError("get", { scope: PACKAGE_SCOPE.value, roots: [toPlainNodeRef(parent)] }).tx;
  const thread = tx.create({
    metatype: NodeType.MESSAGE,
    parentPtr: parent != null ? toPlainNodeRef(parent) : undefined,
    packagePtr,
    title,
  });
  if (self.value != null) {
    const selfView = spaceGraph.getOrError(self.value);
    canvas.tx().update(selfView, { nodePtr: toNodeRefOneOf(thread) });
  } else {
    emit("update:self", { nodePtr: toPlainNodeRef(thread) });
  }
  return thread;
}

/** Submits a message to the current thread. If it doesn't exist, create a root thread. */
function submit() {
  if (isTextEmpty(text.value)) return; // nothing to submit

  if (nodePtr.value == null || !isNode(node.value, NodeType.MESSAGE)) {
    // create new thread with message inside
    const parent = node.value ?? pkg.value;
    if (parent == null) throw new Error("no parent to create thread in");
    const thread = createNewThread(parent);
    const tx = findExistingConnectionOrError("get", {
      scope: PACKAGE_SCOPE.value,
      roots: [toPlainNodeRef(thread)],
    }).tx;
    const message = tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: toPlainNodeRef(thread),
      packagePtr: thread.packagePtr,
      text: text.value,
    });
  } else {
    // append to existing thread
    if (node.value == null) throw new Error(`thread not found: ${describeNode(nodePtr.value)}`);
    if (!isNode(node.value, NodeType.MESSAGE)) throw new Error(`not a message: ${describeNode(node.value)}`);
    pkgConnection.tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: nodePtr.value,
      packagePtr: node.value.packagePtr!,
      replyToPtr: replyingTo.value != null ? toPlainNodeRef(replyingTo.value) : undefined,
      text: text.value,
    });
  }

  // reset
  stickToEnd.value = true;
  scrollRef.value?.scrollToEnd();
  replyingTo.value = null;
  text.value = emptyText();
}

function mapToNode(element: HTMLElement | SVGElement | ViewComponent): NodeReferenceData | null {
  // find 'data-message-id' attribute
  let el = getElement(element);
  while (el != null) {
    const id = el.getAttribute("data-message-id");
    if (id != null) {
      const message = pkgGraph.get({ id, type: NodeType.MESSAGE });
      if (message != null) return toPlainNodeRef(message);
    }
    el = el.parentElement;
  }
  return null;
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (anchor == null || typeof anchor === "string" || anchor.type != NodeType.MESSAGE) {
    textRef.value?.focus?.("center");
  } else {
    const selfView = spaceGraph.getOrError(self.value!);
    canvas.focusInGraph({ view: selfView, focus: makeSelection([anchor]) });
    const messageEl = messageRefs.value[anchor.id!];
    if (messageEl != null) messageEl.scrollIntoView({ behavior: "instant", block: "center" });
  }
}

// actions
const getMessageFromContext = (ctx: ActionContext | undefined): { message: MessageData | null; idx: number } => {
  let message = messages.value.find((item) => item.id == ctx?.triggerNode?.id);
  if (!message) message = messages.value.find((item) => item.id == focusedNodePtr.value?.id);
  if (!message) return { message: null, idx: -1 };
  const idx = messages.value.findIndex((item) => item.id == message?.id);
  return { message, idx };
};
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"message"> = {
  // common
  "common.edit.archive": {
    action: (action, context) => {
      const { message } = getMessageFromContext(context);
      if (message == null) return false;
      pkgConnection.tx.archive(message);
    },
  },
  "common.edit.delete": {
    action: (action, context) => {
      const { message } = getMessageFromContext(context);
      if (message == null) return false;
      pkgConnection.tx.delete(message);
    },
  },
  "common.navigate.up": {
    action: (action, context) => {
      const { message, idx } = getMessageFromContext(context);
      if (message == null) return false;
      if (idx > 0) focus(toPlainNodeRef(messages.value[idx - 1]));
    },
  },
  "common.navigate.down": {
    action: (action, context) => {
      const { message, idx } = getMessageFromContext(context);
      if (message == null) return false;
      if (idx < messages.value.length - 1) focus(toPlainNodeRef(messages.value[idx + 1]));
    },
  },
  // message
  "message.handle.reply": {
    action: (action, context) => {
      const { message } = getMessageFromContext(context);
      if (message == null) return false;
      replyTo(message);
    },
  },
  "message.handle.startThread": {
    isEnabled: () => false, // not yet supported
    action: (action, context) => {
      throw new Error(":Incomplete: start nested thread");
    },
  },
  "message.handle.edit": {
    isEnabled: () => false, // not yet supported
    action: (action, context) => {
      throw new Error(":Incomplete: edit message");
    },
  },
  "message.handle.pin": {
    isChecked: (action, context) => {
      const { message } = getMessageFromContext(context);
      return message?.isPinned ?? false;
    },
    action: (action, context) => {
      const { message } = getMessageFromContext(context);
      if (message == null) return false;
      pkgConnection.tx.update(message, { isPinned: !message.isPinned });
    },
  },
};

const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.COMPACT], mapToNode, actions, focus });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Header -->
    <div class="w-full" :style="{ height: headerHeight + 'px' }">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center gap-x-3 px-4"
        :style="{ height: headerHeight + 'px', minWidth: MIN_WIDTH + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Thread (local root message node) -->
        <div class="flex flex-shrink-0 flex-row items-center">
          <!-- Context (path) -->
          <template v-if="context != null && context?.metatype != ObjectType.PACKAGE && variant != Variant.COMPACT">
            <NodePath class="flex-shrink-0" :container="toPlainNodeRef(context)" :graph="pkgGraph" />
            <i class="fas fa-chevron-right ml-1 mr-1.5 text-gray-400" />
          </template>
          <!-- Icon / Name -->
          <i
            class="fas fa-message mr-1.5 w-5 text-center"
            :class="[thread == null ? 'text-gray-400' : 'text-gray-700']"
          />
          <input
            class="truncate rounded border-0 bg-transparent py-0.5 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
            :class="[thread == null ? 'text-gray-400' : 'text-gray-900', thread?.title != null ? 'font-medium' : '']"
            spellcheck="false"
            :value="thread?.title"
            :size="(thread?.title?.length ?? 10) + 1"
            :disabled="thread == null"
            :placeholder="thread == null ? 'New Thread' : 'Untitled Thread'"
            v-bind="getNativeConstraintProps(TITLE_CONSTRAINT)"
            @input="
              guardNativeInput(TITLE_CONSTRAINT, $event, thread?.title, (newValue) =>
                pkgConnection.tx.update(node!, { title: newValue }, { debounce: 'long' }),
              )
            "
          />
          <!-- Select thread -->
          <button
            v-menu="
              (): PopoverInfoIn => ({
                component: ViewType.PICKER,
                placement: 'bottom-left',
                container: 'containingRoot',
                props: {
                  valueType: makeTypeInfo({ benchType: BenchType.MESSAGE }),
                  modelValue: nodePtr,
                  placeholder: 'Select Thread',
                  customIndex: graphIndex({
                    id: 'graph',
                    graph: nodePtr == null ? localPkgGraph : pkgGraph,
                    metatypes: [NodeType.MESSAGE],
                    roots: [context!],
                    skipDepth: 1,
                    maxDepth: 1,
                  }),
                } as any,
                onApply: (value) => {
                  if (value != null) {
                    const selfView = spaceGraph.getOrError(self!);
                    canvas.tx().update(selfView, { nodePtr: toNodeRefOneOf(value) });
                    $nextTick(followEnd);
                  }
                },
              })
            "
            class="ml-1.5 text-gray-400 hover:text-primary-900"
          >
            <i class="fas fa-chevron-down" />
          </button>
        </div>
        <!-- Actions / Menu -->
        <div class="ml-auto flex flex-shrink-0 flex-row gap-x-1">
          <!-- Expand into own View -->
          <button
            v-if="variant == Variant.COMPACT"
            class="text-gray-400 enabled:hover:text-primary-900"
            @click="
              () => {
                canvas.addView(
                  { type: ViewType.CHAT, nodePtr: props.nodePtr },
                  { ifPresent: 'upsertAndFocus', where: 'bestFrame' },
                );
                $emit('close');
              }
            "
          >
            <i class="fas fa-expand w-5 text-center" />
          </button>
          <!-- Search -->
          <button v-if="variant != Variant.COMPACT" class="text-gray-400 enabled:hover:text-primary-900">
            <i class="fas fa-magnifying-glass w-5 text-center" />
          </button>
        </div>
      </div>
    </div>

    <!-- Body -->
    <Scroll
      v-if="thread != null && renderedMessages.length > 0"
      ref="scrollRef"
      :size="{
        width: props.size?.width ?? DEFAULT_WIDTH,
        height: (props.size?.height ?? DEFAULT_HEIGHT) - headerHeight - inputSize.height.value,
      }"
      :orientation="Orientation.VERTICAL"
      :track-width="variant == Variant.COMPACT ? ScrollbarWidth.sm : ScrollbarWidth.md"
      :track-is-overlay="variant == Variant.COMPACT"
      :stick-to-end="stickToEnd"
      :size-is-dynamic="props.size == null"
    >
      <!-- Messages -->
      <div class="" :class="variant != Variant.COMPACT ? 'my-2' : 'my-0.5'">
        <!-- Message -->
        <div
          v-for="{
            message,
            author,
            isContinued,
            isContinuationBreak,
            replyToMessage,
            replyToAuthor,
          } in renderedMessages"
          :key="message.id"
          :ref="(ref?: any) => (ref != null ? (messageRefs[message.id] = ref) : delete messageRefs[message.id])"
          v-contextmenu="
            (context: PopoverContext): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-right',
              offset: 'referenceWidth',
              items: menuActionsLike(['message.*', 'common.edit.delete'], {
                context: { ...context, triggerNode: message },
              }),
            })
          "
          :data-message-id="message.id"
          class="group/message relative mx-auto flex max-w-full flex-row gap-x-2.5 rounded py-0.5"
          :class="[
            isContinuationBreak ? 'mt-2' : '',
            replyingTo?.id == message.id ? 'bg-secondary-100' : '',
            variant != Variant.COMPACT ? 'px-3' : 'px-1',
          ]"
          :style="{ width: 'calc(100% - ' + MIN_GUTTER_WIDTH * 2 + 'px)', maxWidth: MAX_WIDTH + 'px' }"
        >
          <!-- Handle -->
          <div
            class="-mr-1 rounded transition-colors duration-75"
            :style="{ width: HANDLE_WIDTH + 'px' }"
            :class="
              message.id == inspectionPtr?.id
                ? 'bg-primary-900'
                : message.id == focusedNodePtr?.id
                  ? isFocusedAbsolute
                    ? 'bg-primary-900'
                    : 'bg-gray-300'
                  : 'bg-transparent group-hover/message:bg-gray-200'
            "
          />

          <!-- Aside -->
          <!-- Author Icon -->
          <div
            v-if="!isContinued"
            class="mt-0.5 flex h-fit flex-shrink-0 flex-col justify-center rounded border border-gray-200 bg-gray-100 text-center text-gray-700"
            :style="{ width: asideSize + 'px', height: asideSize + 'px' }"
          >
            <IconInline v-bind="author.icon" />
          </div>
          <!-- Time (if continued) -->
          <div v-else class="flex-shrink-0 px-0.5" :style="{ width: asideSize + 'px' }">
            <span class="text-xs text-gray-400 opacity-0 transition-colors duration-75 group-hover/message:opacity-100">
              {{ tsToDt(message.createdAt!).toLocaleString(DateTime.TIME_24_SIMPLE) }}
            </span>
          </div>

          <!-- Body -->
          <div class="w-full">
            <!-- Header -->
            <div v-if="!isContinued" class="mb-0.5 max-w-full gap-x-0.5">
              <!-- Author Name / Time -->
              <span class="truncate font-medium">{{ author.name }}</span>
              <span class="ml-1.5 text-xs text-gray-400">{{ formatAbsoluteDate(message.createdAt!) }}</span>
              <i v-if="message.isPinned" class="fas fa-thumbtack ml-1.5 text-xs text-gray-400" />
            </div>
            <!-- Reply to -->
            <div
              v-if="replyToMessage"
              role="button"
              class="my-0.5 rounded border-secondary-200 bg-gray-100 px-2 py-1 hover:cursor-pointer"
              :style="{ borderLeftWidth: HANDLE_WIDTH + 'px' }"
              @click="focus(toPlainNodeRef(replyToMessage))"
            >
              <span class="truncate font-medium text-secondary-900">{{ replyToAuthor!.name }}</span>
              <span class="ml-1.5 text-xs text-gray-400">{{ formatAbsoluteDate(replyToMessage.createdAt!) }}</span>
              <Text
                v-if="replyToMessage.text"
                class="max-h-6 max-w-full select-none truncate hover:cursor-pointer"
                :model-value="trimText(replyToMessage.text, 1)"
                :variant="Variant.STEALTH"
              />
            </div>
            <!-- Content -->
            <Text :model-value="message.text" :variant="Variant.STEALTH" />
            <!-- Controls (floating) -->
            <div
              class="absolute right-1.5 top-0 z-10 ml-auto flex flex-row gap-x-2 rounded border border-gray-200 bg-white px-2 py-1 opacity-0 group-hover/message:opacity-100"
            >
              <!-- Reply -->
              <button
                v-tooltip="{ title: 'Reply', referenceMargin: 8, small: true, showDelay: 200, hideDelay: 100 }"
                class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900"
                @click.stop="replyTo(message)"
              >
                <i class="fas fa-reply w-5 text-center" />
              </button>
              <!-- Thread (not supported yet) -->
              <button
                v-tooltip="{ title: 'Thread', referenceMargin: 8, small: true, showDelay: 200, hideDelay: 100 }"
                class="rounded text-gray-300"
              >
                <i class="fas fa-reel w-5 text-center" />
              </button>
              <!-- Menu -->
              <button
                v-menu="
                  (context: PopoverContext): PopoverInfo => ({
                    kind: 'menu',
                    placement: 'bottom-left',
                    offset: 'referenceWidth',
                    items: menuActionsLike(['message.*', 'common.edit.archive', 'common.edit.delete'], {
                      context: { ...context, triggerNode: message },
                    }),
                  })
                "
                class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900"
              >
                <i class="fas fa-ellipsis-v w-5 text-center" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </Scroll>
    <!-- Can't find thread -->
    <Inaccessible
      v-else-if="nodePtr != null && nodePtr.type == NodeType.MESSAGE"
      class="my-1 h-full w-full"
      :node="nodePtr"
      :connection="pkgConnection"
    />

    <!-- Draft area -->
    <div ref="draftRef" class="group mt-auto" @click="() => textRef?.focus?.()">
      <!-- Replying to -->
      <div
        v-if="replyingTo"
        class="mx-auto mt-2 flex flex-row rounded-t bg-secondary-100 px-5 py-1.5"
        :style="{ width: 'calc(100% - ' + MIN_GUTTER_WIDTH * 2 + 'px)', maxWidth: MAX_WIDTH + 'px' }"
      >
        <span class="text-gray-700"
          >Replying to
          <span class="font-medium text-gray-900">{{ getAuthor(replyingTo).name }}</span>
        </span>
        <button class="ml-auto text-gray-400 hover:text-primary-900" @click="replyingTo = null">
          <i class="fas fa-xmark w-5 text-center" />
        </button>
      </div>

      <!-- Create message -->
      <div
        class="relative mx-auto flex flex-row items-end"
        :class="[
          variant != Variant.COMPACT ? 'mb-3 gap-x-2.5 bg-gray-100 px-3 py-2' : 'gap-x-1.5 px-2 pb-1.5 pt-1',
          replyingTo != null ? 'rounded-b' : 'rounded',
        ]"
        :style="{
          // all these magic numbers just make sure that things are nicely aligned
          width: 'calc(100% - ' + (MIN_GUTTER_WIDTH * 2 + 26) + 'px)',
          maxWidth: MAX_WIDTH + 'px',
          minHeight: MIN_INPUT_HEIGHT + 'px',
        }"
      >
        <!-- Jump to bottom & follow -->
        <button
          v-if="variant != Variant.COMPACT"
          class="arrow duation-75 absolute right-[12px] rounded-2xl border border-gray-200 bg-white px-2.5 py-0.5 text-base text-gray-600 transition-colors hover:bg-gray-100 hover:text-primary-900"
          :class="[replyingTo ? '-top-[72px]' : '-top-[36px]', stickToEnd ? 'opacity-0' : 'opacity-100']"
          @click="followEnd()"
        >
          <i class="fas fa-arrow-down" />
        </button>

        <!-- Upload/create -->
        <button
          class="h-fit self-end px-2 py-0.5"
          disabled
          :class="[
            isFocusedAbsolute || variant == Variant.COMPACT
              ? 'enabled:text-gray-600 enabled:hover:text-primary-900 disabled:text-gray-400'
              : 'text-gray-400',
            variant != Variant.COMPACT ? 'text-lg' : 'text-base',
          ]"
          :style="{ width: asideSize + 'px' }"
          @click.stop="() => {}"
        >
          <i class="fas fa-plus-circle" />
        </button>
        <!-- Content -->
        <Scroll
          :size="{ width: size?.width ?? DEFAULT_WIDTH, height: maxInputHeight }"
          size-is-dynamic
          :orientation="Orientation.VERTICAL"
          track-is-overlay
          :track-width="ScrollbarWidth.sm"
          class="w-full"
          :class="variant != Variant.COMPACT ? '' : 'mx-[1px]'"
        >
          <Text
            ref="textRef"
            v-model="text"
            class="w-full self-end hover:cursor-text"
            :class="variant != Variant.COMPACT ? 'my-1 ' : 'my-0.5'"
            :variant="Variant.STEALTH"
            is-input
            :placeholder="
              context != null && context.metatype != ObjectType.PACKAGE
                ? `Message this ${toCamelName(NodeType, context.metatype)}`
                : `Message your Bench`
            "
            suppress-enter
            @click.stop
            @keydown.enter.exact.stop="submit"
          />
        </Scroll>
        <!-- Submit -->
        <button
          class="h-fit self-end px-2 py-0.5"
          :class="[
            isFocusedAbsolute || variant == Variant.COMPACT
              ? 'enabled:text-gray-600 enabled:hover:text-primary-900 disabled:text-gray-400'
              : 'text-gray-400',
            variant != Variant.COMPACT ? 'text-lg' : 'text-base',
          ]"
          :style="{ width: asideSize + 'px' }"
          :disabled="!canSubmit"
          @click.stop="submit"
        >
          <i class="fas fa-circle-arrow-up" />
        </button>
      </div>
    </div>
  </div>
</template>
