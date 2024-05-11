<script lang="ts" setup>
import {
  ViewData,
  NodeType,
  BoxData,
  Variant,
  ObjectType,
  Orientation,
  TextData,
  MessageData,
  ViewType,
  BenchType,
  NodeReferenceData,
  type AnyNodeData,
  IconData,
} from "@/proto/wire";
import { describeNode, isNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type FocusAnchor, type ViewComponent, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr, pkg, pkgGraph as localPkgGraph } from "@/system/space";
import { computed, ref, toRef, watch, type Ref } from "vue";
import { findExistingConnectionOrError, useExistingConnection } from "@/system/connection";
import Scroll from "@/views/containers/Scroll.vue";
import { ScrollbarWidth } from "@/utils/layout";
import { useElementSize } from "@vueuse/core";
import Text from "@/views/content/Text.vue";
import { DEFAULT_USER_ICON, IconInline, getNodeIcon, makeIcon } from "@/system/icon";
import { emptyText, isTextEmpty, trimText } from "@/system/text";
import { formatAbsoluteDate, tsToDt } from "@/utils/time";
import { user } from "@/system/user";
import { DateTime } from "luxon";
import { generateRandomName } from "@/utils/naming";
import { menuActionsLike, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/utils/menu";
import { makeTypeInfo } from "@/system/value";
import { graphIndex } from "@/system/search";
import { computedValue } from "@/utils/ref";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeSelection } from "@/views/canvas";
import { getElement } from "@/utils/element";
import { toCamelName } from "@/system/lang";

const HEADER_HEIGHT = 36;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 320;
const DEFAULT_HEIGHT = 480;
const DEFAULT_MAX_INPUT_HEIGHT = 120;
const MIN_INPUT_HEIGHT = 40;
const MIN_GUTTER_WIDTH = 12;
const ASIDE_WIDTH = 36;
const HANDLE_WIDTH = 6;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; size?: Required<Pick<BoxData, "width" | "height">> } & Partial<
    Pick<ViewData, "title" | "nodePtr" | "focus" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const maxInputHeight = computed(() =>
  props.size != null ? (props.size.height - HEADER_HEIGHT) / 2 : DEFAULT_MAX_INPUT_HEIGHT,
);
const draftRef: Ref<HTMLDivElement | null> = ref(null);
const inputSize = useElementSize(draftRef);
const textRef: Ref<InstanceType<typeof Text> | null> = ref(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const messageRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

// NOTE: threadPtr can point to a message node if we already have a thread or to any node to create a thread on
const nodePtr = toRef(props, "nodePtr");
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const preparedPkgConnection = useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const node = pkgGraph.getRef(nodePtr);
const thread = computed(() => (node.value != null && isNode(node.value, NodeType.MESSAGE) ? node.value : null));
const messages = pkgGraph.getChildrenRef(node, NodeType.MESSAGE);
const ancestors = pkgGraph.getAncestorsRef(nodePtr, { includeSelf: false });
const context = computed(() => {
  if (node.value != null && !isNode(node.value, NodeType.MESSAGE)) return node.value;
  else return ancestors.value.find((n) => n.metatype != ObjectType.MESSAGE);
});

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
      message.createdAt!.seconds - lastMessage.createdAt!.seconds < 300 &&
      message.replyToPtr == null;
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

function createNewThread(parent: AnyNodeData | null, title: string = generateRandomName()) {
  if (pkg.value == null) throw new Error("no package");
  const rootPtr = toNodeReference(pkg.value);
  const tx = findExistingConnectionOrError("get", { roots: [rootPtr] }).tx;
  const thread = tx.create({
    metatype: NodeType.MESSAGE,
    parentPtr: rootPtr,
    packagePtr: rootPtr,
    title,
  });
  if (self.value != null) {
    const selfView = spaceGraph.getOrError(self.value);
    spaceConnection.tx.update(selfView, { nodePtr: toNodeReference(thread) });
  } else {
    // nocheckin: handle update:self in PopoverOverlay
    emit("update:self", { nodePtr: toNodeReference(thread) });
  }
  return thread;
}

/** Submits a message to the current thread. If it doesn't exist, create a root thread. */
function submit() {
  if (isTextEmpty(text.value)) return;

  if (nodePtr.value == null || !isNode(node.value, NodeType.MESSAGE)) {
    // create new thread with message inside (in node or default to package)
    const parent = isNode(nodePtr.value, NodeType.MESSAGE) ? nodePtr.value : null;
    const thread = createNewThread(parent);
    const tx = findExistingConnectionOrError("get", { roots: [toNodeReference(thread)] }).tx;
    const message = tx.create({
      metatype: NodeType.MESSAGE,
      parentPtr: toNodeReference(thread),
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
      replyToPtr: replyingTo.value != null ? toNodeReference(replyingTo.value) : undefined,
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
      if (message != null) return toNodeReference(message);
    }
    el = el.parentElement;
  }
  return null;
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (anchor == null || typeof anchor === "string") {
    textRef.value?.focus?.("center");
  } else {
    if (anchor.type != NodeType.MESSAGE) throw new Error(`can't focus non-message: ${describeNode(anchor)}`);
    const selfView = spaceGraph.getOrError(self.value!);
    spaceConnection.tx.updateDebounced(selfView, { focus: makeSelection([anchor]) });
    const messageEl = messageRefs.value[anchor.id!];
    if (messageEl != null) messageEl.scrollIntoView({ behavior: "smooth", block: "center" });
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
      pkgConnection.tx.softDelete(message);
    },
  },
  "common.navigate.up": {
    action: (action, context) => {
      const { message, idx } = getMessageFromContext(context);
      if (message == null) return false;
      if (idx > 0) focus(toNodeReference(messages.value[idx - 1]));
    },
  },
  "common.navigate.down": {
    action: (action, context) => {
      const { message, idx } = getMessageFromContext(context);
      if (message == null) return false;
      if (idx < messages.value.length - 1) focus(toNodeReference(messages.value[idx + 1]));
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
      throw new Error(":Incomplete: start thread");
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
    <div v-if="variant != Variant.COMPACT" class="w-full border-b border-gray-200">
      <div
        class="mx-auto flex w-full max-w-full flex-row items-center gap-x-3 pl-4 pr-5"
        :style="{ height: HEADER_HEIGHT + 'px', maxWidth: MAX_WIDTH + 'px' }"
      >
        <!-- Thread (local root message node) -->
        <div class="flex-shrink-0">
          <i
            class="fas fa-message mr-1.5 w-5 text-center"
            :class="[node == null ? 'text-gray-500' : 'text-gray-700']"
          />
          <input
            class="truncate rounded border-0 py-0.5 outline-none ring-0 hover:bg-gray-100 focus:ring-0"
            :class="[node == null ? 'text-gray-600' : 'text-gray-900', thread?.title != null ? 'font-medium' : '']"
            spellcheck="false"
            :value="thread?.title"
            :size="(thread?.title?.length ?? 10) + 1"
            :disabled="node == null"
            :placeholder="node == null ? 'New Thread' : 'Untitled Thread'"
            @input="
              (event) => pkgConnection.tx.updateDebounced(node!, { title: (event.target as HTMLInputElement).value })
            "
          />
          <!-- Select thread -->
          <button
            v-menu="
              (): PopoverInfoIn => ({
                component: ViewType.PICKER,
                placement: 'bottom-left',
                props: {
                  valueType: makeTypeInfo({ benchType: BenchType.MESSAGE }),
                  modelValue: nodePtr,
                  placeholder: 'Select Thread',
                  customIndex: graphIndex({
                    graph: nodePtr == null ? localPkgGraph : pkgGraph,
                    metatypes: [NodeType.MESSAGE],
                    roots: [pkg!],
                    filter: (node) => (node as MessageData).parentPtr!.type != NodeType.MESSAGE,
                  }),
                } as any,
                onApply: (value) => {
                  if (value != null) {
                    const selfView = spaceGraph.getOrError(self!);
                    spaceConnection.tx.update(selfView, { nodePtr: value });
                    $nextTick(followEnd);
                  }
                },
              })
            "
            class="ml-1.5 text-gray-400"
          >
            <i class="fas fa-chevron-down" />
          </button>
        </div>
        <!-- Archive / delete -->
        <button
          v-if="thread != null"
          v-tooltip="{ title: 'Archive', small: true }"
          class="text-gray-400 enabled:hover:text-primary-900"
          :disabled="thread == null"
          @click="
            () => {
              pkgConnection.tx.archive(thread!);
              const selfView = spaceGraph.getOrError(self!);
              spaceConnection.tx.updateDebounced(selfView, { nodePtr: undefined });
            }
          "
        >
          <i class="fas fa-archive w-5 text-center" />
        </button>
        <!-- Context (non-message parent node) -->
        <div class="ml-auto flex-shrink-0">
          <IconInline
            class="mr-1.5 w-5 text-center text-gray-400"
            v-bind="(context != null ? getNodeIcon(context) : null) ?? makeIcon('fas fa-infinity')"
          />
          <span class="text-gray-400">
            {{ (context as any)?.name ?? "Everything" }}
          </span>
          <!-- TODO :UX: move thread to different parent ('context') -->
        </div>
      </div>
    </div>

    <!-- Body -->
    <Scroll
      v-if="node && renderedMessages.length > 0"
      ref="scrollRef"
      :size="{
        width: props.size?.width ?? DEFAULT_WIDTH,
        height: (props.size?.height ?? DEFAULT_HEIGHT) - HEADER_HEIGHT - inputSize.height.value,
      }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      :stick-to-end="stickToEnd"
      :size-is-dynamic="props.size == null"
    >
      <!-- Messages -->
      <div class="my-2">
        <template
          v-for="{
            message,
            author,
            isContinued,
            isContinuationBreak,
            replyToMessage,
            replyToAuthor,
          } in renderedMessages"
          :key="message.id"
        >
          <!-- Message -->
          <div
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
            class="group/message relative mx-auto flex max-w-full flex-row rounded px-2 py-0.5"
            :class="[isContinuationBreak ? 'mt-2' : '', replyingTo?.id == message.id ? 'bg-secondary-100' : '']"
            :style="{ width: 'calc(100% - ' + MIN_GUTTER_WIDTH * 2 + 'px)', maxWidth: MAX_WIDTH + 'px' }"
          >
            <!-- Handle -->
            <div
              class="mr-2 rounded transition-colors duration-75"
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
              class="mr-3 mt-0.5 h-fit flex-shrink-0 rounded border border-gray-200 bg-gray-100 py-1.5 text-center text-gray-700"
              :style="{ width: ASIDE_WIDTH + 'px' }"
            >
              <IconInline v-bind="author.icon" />
            </div>
            <!-- Time (if continued) -->
            <div v-else class="mr-3.5 flex-shrink-0 px-0.5" :style="{ width: ASIDE_WIDTH + 'px' }">
              <span
                class="text-xs text-gray-400 opacity-0 transition-colors duration-75 group-hover/message:opacity-100"
              >
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
              </div>
              <!-- Reply to -->
              <div
                v-if="replyToMessage"
                role="button"
                class="my-0.5 rounded border-secondary-200 bg-gray-100 px-2 py-1 hover:cursor-pointer"
                :style="{ borderLeftWidth: HANDLE_WIDTH + 'px' }"
                @click="focus(toNodeReference(replyToMessage))"
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
                      items: menuActionsLike(['message.*', 'common.edit.delete'], {
                        context: { ...context, triggerNode: nodePtr },
                      }),
                    })
                  "
                  class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:border-primary-900"
                >
                  <i class="fas fa-ellipsis-v w-5 text-center" />
                </button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </Scroll>
    <!-- Can't find thread -->
    <Inaccessible
      v-else-if="nodePtr != null && nodePtr.type == NodeType.MESSAGE"
      class="h-full w-full"
      :node="nodePtr"
      :is-connected="pkgConnection.isConnected.value"
    />

    <!-- Draft area -->
    <div ref="draftRef" class="group mt-auto" @click="textRef?.focus">
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
          variant != Variant.COMPACT ? 'mb-3 gap-x-2.5 bg-gray-100 px-3 py-2' : 'gap-x-1.5 px-2.5 pb-1.5 pt-1',
          replyingTo != null ? 'rounded-b' : 'rounded',
        ]"
        :style="{
          width: variant != Variant.COMPACT ? 'calc(100% - ' + MIN_GUTTER_WIDTH * 2 + 'px)' : DEFAULT_WIDTH + 'px',
          maxWidth: MAX_WIDTH + 'px',
          minHeight: MIN_INPUT_HEIGHT + 'px',
        }"
      >
        <!-- Jump to bottom & follow -->
        <button
          v-if="!stickToEnd"
          class="arrow absolute right-[12px] rounded-2xl border border-gray-200 bg-white px-2.5 py-0.5 text-base text-gray-600 hover:bg-gray-100 hover:text-primary-900"
          :class="replyingTo ? '-top-[72px]' : '-top-[36px]'"
          @click="followEnd()"
        >
          <i class="fas fa-arrow-down" />
        </button>

        <!-- Upload/create -->
        <button
          class="h-fit self-end px-2 py-0.5"
          :class="[
            isFocusedAbsolute ? 'text-gray-600 hover:text-primary-900' : 'text-gray-400 group-hover:text-gray-500',
            variant != Variant.COMPACT ? 'text-lg' : 'text-base',
          ]"
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
            isFocusedAbsolute
              ? 'enabled:text-gray-600 enabled:hover:text-primary-900 disabled:text-gray-400'
              : 'text-gray-400 group-hover:text-gray-500',
            variant != Variant.COMPACT ? 'text-lg' : 'text-base',
          ]"
          :disabled="!canSubmit"
          @click.stop="submit"
        >
          <i class="fas fa-circle-arrow-up" />
        </button>
      </div>
    </div>
  </div>
</template>
