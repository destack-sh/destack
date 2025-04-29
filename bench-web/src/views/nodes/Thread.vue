<script lang="ts" setup>
import { supergraph } from "@/globals";
import { BENCH_BENCH_AGENT_PTR, BENCH_BENCH_UBUNTU_DESKTOP_PTR } from "@/language/core/builtin";
import { isProcessableNode, toCamelName } from "@/language/core/const";
import { getEnumOption, getEnumOptions } from "@/language/core/enum";
import { makeAndConditional, makeExpression } from "@/language/core/expression";
import { emptyText, isTextEmpty, renderText, trimText } from "@/language/core/text";
import { newChangeId } from "@/language/core/transaction";
import { INLINABLE_FILE_TYPES, uploadFile } from "@/language/resource/file";
import { VERB_BY_RESOURCE_STATUS } from "@/language/resource/resource";
import { isProcessActive, touchProcess } from "@/language/runtime/process";
import { createClaim } from "@/language/source/claim";
import { createThread } from "@/language/source/thread";
import { createMessage, getMessageAuthorPtr } from "@/language/state/message";
import {
  Alignment,
  AnyNodeData,
  ClaimType,
  ColorType,
  ComputerData,
  EnumType,
  ExpressionData,
  ExpressionType,
  IconData,
  MessageData,
  MessageProperty,
  MessageType,
  MessageTypeOptionInfo,
  NodeMode,
  NodeReferenceData,
  NodeType,
  Orientation,
  RectangleData,
  ResourceStatus,
  SubjectNodeData,
  TextData,
  ThreadData,
  Timestamp,
  ViewData,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr, CURRENT_BENCH_SCOPE, packagePtr } from "@/system/client";
import { SearchConnectionParams, useAutoConnection, useInfiniteSearchConnection } from "@/system/connection";
import { bench, benchConnection, benchGraph, canvas, pkg, space } from "@/system/space";
import { user } from "@/system/user";
import { CommandMapKit, fireCommand, getCommand, getNodesForCommand, MESSAGE_CONTEXT_COMMANDS } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone, useSingleDropZone } from "@/ui/drag";
import { AvatarInline, getNodeIcon, getNodeTitle, IconInline, makeIcon } from "@/ui/icon";
import { getNodeColor } from "@/ui/style";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { formatAbsoluteDate, getNow, TimeUpdateInterval, tsToDt } from "@/utils/time";
import Link from "@/views/builtin/Link.vue";
import NodeReference from "@/views/builtin/NodeReference.vue";
import Popover from "@/views/builtin/Popover.vue";
import RootHeader from "@/views/builtin/RootHeader.vue";
import Run from "@/views/builtin/Run.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import Text from "@/views/content/Text.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { useElementSize, useElementVisibility, useEventListener } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, nextTick, Ref, ref, toRef, watch, watchEffect } from "vue";

const MAX_THREAD_WIDTH = 1000;
const MIN_GUTTER_WIDTH = 30;
const LOADING_SKELETON_COUNT = 3;
const CHUNK_SIZE = 80;
const MIN_AUTOSCROLL_INTERVAL_MILLISECONDS = 500;
const MESSAGE_GROUP_TIME_SECONDS = 5 * 60; // 5 minutes
const MESSAGE_SIDE_WIDTH = 52;
const MESSAGE_MAX_LINES = 60;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    isRoot?: boolean;
    size: Required<Pick<RectangleData, "width" | "height">>;
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focusPtr" | "alignment" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const alignment = computed(() => props.alignment ?? Alignment.END);
canvas.registerView(self, id);

// state
const nodePtr = toRef(props, "nodePtr");
const { node, connection, graph } = supergraph.getLinkRef(nodePtr, { excludeSearch: true });
const thread = node as Ref<ThreadData | null>;
const { graph: threadGraph, connection: threadConnection } = useAutoConnection(nodePtr); // ughh
const cursors = threadGraph.getChildrenRef(nodePtr, NodeType.CURSOR);
const runs = threadGraph.getChildrenRef(nodePtr, NodeType.RUN);
const claims = threadGraph.getChildrenRef(nodePtr, NodeType.CLAIM);
const computers = threadGraph.getChildrenRef(nodePtr, NodeType.COMPUTER);
const hasComputerClaim = computed(() =>
  claims.value.some(
    (c) => c.targetPtr?.nodeType == NodeType.COMPUTER || c.targetTemplatePtr?.nodeType == NodeType.COMPUTER,
  ),
);
const computer: Ref<ComputerData | null> = computed(() => computers.value[0] ?? null);
const hasClaimedComputer = ref(false);
const CONTEXT_TABS = ["File", "Pages"];
const contextTab: Ref<(typeof CONTEXT_TABS)[number]> = ref(CONTEXT_TABS[0]);

// size block/gutter horizontally (try to fit both until min block width)
const widths = computed(() => {
  // always respect MIN_GUTTER_WIDTH first
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - MAX_THREAD_WIDTH) / 2);
  // calculate body width with the remaining space
  const bodyWidth = Math.min(MAX_THREAD_WIDTH, props.size.width - gutterWidth * 2);
  const textWidth = bodyWidth - MESSAGE_SIDE_WIDTH;
  return { body: bodyWidth, text: textWidth, gutter: gutterWidth };
});

//
// Messages
//

const isEnabled = computed(() => nodePtr.value != null);
const filter = computed(() => {
  const filters: ExpressionData[] = [];
  // thread
  if (nodePtr.value != null) {
    filters.push(
      makeExpression({
        type: ExpressionType.EQUALS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.threadPtr),
        value: nodePtr.value,
      }),
    );
  } else {
    filters.push(
      makeExpression({
        type: ExpressionType.NOT_EXISTS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.threadPtr),
      }),
    );
  }
  return makeAndConditional(filters);
});
const {
  roots: messages,
  txFactory,
  isAtStart,
  isAtEnd,
  isConnected,
  go,
} = useInfiniteSearchConnection(
  { name: "chat", live: true },
  computed(
    (): SearchConnectionParams<NodeType.MESSAGE> => ({
      scope: CURRENT_BENCH_SCOPE.value,
      nodeType: NodeType.MESSAGE,
      count: true,
      filter: filter.value,
    }),
  ),
  {
    nodeType: NodeType.MESSAGE,
    chunkSize: CHUNK_SIZE,
    isEnabled,
    onAdded: () => {
      // auto-scroll to end if we're at the end
      if (isAtEnd.value) {
        stickToEnd.value = true;
        bodyScrollRef.value?.scrollToEnd();
      }
    },
  },
);
const topPlaceholderRef = ref<InstanceType<typeof HTMLDivElement> | null>(null);
const bottomPlaceholderRef = ref<InstanceType<typeof HTMLDivElement> | null>(null);
const topPlaceholderVisible = useElementVisibility(topPlaceholderRef);
const bottomPlaceholderVisible = useElementVisibility(bottomPlaceholderRef);

// auto scroll up/down
const now = getNow(TimeUpdateInterval.SECOND);
const lastAutoscrollAt: Ref<DateTime> = ref(now.value.plus({ seconds: 1 })); // don't autoscroll immediately
watchEffect(() => {
  const millisecondsSinceLastAutoscroll =
    lastAutoscrollAt.value != null ? now.value.diff(lastAutoscrollAt.value, "milliseconds").milliseconds : Infinity;
  if (
    isConnected.value &&
    messages.value.length > 0 &&
    millisecondsSinceLastAutoscroll >= MIN_AUTOSCROLL_INTERVAL_MILLISECONDS
  ) {
    if ((topPlaceholderVisible.value || bodyScrollRef.value?.isCloseToStart) && !isAtStart.value) {
      go("up");
      lastAutoscrollAt.value = DateTime.now();
    } else if ((bottomPlaceholderVisible.value || bodyScrollRef.value?.isCloseToEnd) && !isAtEnd.value) {
      go("down");
      lastAutoscrollAt.value = DateTime.now();
    }
  }
});

//
// Views
//

// authors
type AuthorInfo = {
  node: SubjectNodeData;
  icon: IconData | null;
  name: string | null;
  color: ColorType | null;
  isActive: boolean;
};
function getAuthorInfo(node: SubjectNodeData): AuthorInfo {
  const icon = getNodeIcon(node) ?? null;
  const name = getNodeTitle(node) ?? null;
  const color = getNodeColor(node) ?? null;
  let isActive = false;
  if (isNode(node, NodeType.AGENT)) {
    isActive = runs.value.some((r) => r.agentPtr?.id == node.id && isProcessActive(r));
  }

  return {
    node,
    icon,
    name,
    color,
    isActive,
  };
}
const authorsPtr: Ref<NodeReferenceData[]> = computed(() => {
  const authorsPtrById: Record<string, NodeReferenceData> = {};
  for (const message of messages.value) {
    if (message.createdByPtr != null) {
      authorsPtrById[message.createdByPtr.id!] = message.createdByPtr;
    }
  }
  return Object.values(authorsPtrById);
});
const authors = supergraph.getManyRef(authorsPtr, { excludeSearch: true }) as Ref<SubjectNodeData[]>;
// const cursorsPtr = ...
const authorsById: Ref<Record<string, AuthorInfo>> = computed(() =>
  authors.value.reduce(
    (acc, author) => {
      acc[author.id!] = getAuthorInfo(author);
      return acc;
    },
    {} as Record<string, AuthorInfo>,
  ),
);
const activeAuthors = computed(() => Object.values(authorsById.value).filter((a) => a.isActive));

// messages
const expandedMessageIds = ref<string[]>([]);
type MessageView = {
  idx: number;
  message: MessageData;
  nodesPtr: NodeReferenceData[];
  author: AuthorInfo | null;
  replyTo: MessageView | null;
  isEmpty: boolean;
  isStartOfGroup: boolean;
  isEndOfGroup: boolean;
  isCustomLineOnly: boolean;
  isNewDate: boolean;
  isEdited: boolean;
  isEditing: boolean;
  isReplyingTo: boolean;
  isSelected: boolean;
  isOverflowing: boolean;
};
const messageViews = computed(() => {
  const views: MessageView[] = [];
  const viewsById: Record<string, MessageView> = {};
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const prevView = views[i - 1];
    const authorPtr = getMessageAuthorPtr(message);
    const author = authorPtr != null ? authorsById.value[authorPtr.id!] : null;
    const isCustomLineOnly =
      message.type == MessageType.JOIN || message.type == MessageType.LEAVE || message.type == MessageType.RESOURCE;
    let isStartOfGroup;
    let isNewDate;
    if (i == 0) {
      isStartOfGroup = message.createdByPtr != null && !isCustomLineOnly;
      isNewDate = true;
    } else {
      const previousDt = tsToDt(messages.value[i - 1].createdAt!);
      const currentDt = tsToDt(message.createdAt!);
      isStartOfGroup =
        !(isCustomLineOnly && prevView.isCustomLineOnly) &&
        (message.createdByPtr?.id != messages.value[i - 1]?.createdByPtr?.id ||
          prevView.isCustomLineOnly ||
          Math.abs(Number(messages.value[i - 1].createdAt!.seconds) - Number(message.createdAt!.seconds)) >
            MESSAGE_GROUP_TIME_SECONDS);
      isNewDate = previousDt.day != currentDt.day;
    }
    const isEmpty = message.text == null || isTextEmpty(message.text);
    const isEdited = message.editedAt != null && message.editedAt.seconds != message.createdAt?.seconds;
    const isEditing = editingPtr.value?.id == message.id;
    const isReplyingTo = draftReplyTo.value?.id == message.id;
    const isSelected = canvas.isSelected(message);
    const isOverflowing =
      message.text != null &&
      message.text.lines.length > MESSAGE_MAX_LINES &&
      !expandedMessageIds.value.includes(message.id);
    const richMessage: MessageView = {
      idx: i,
      message,
      nodesPtr: message.nodesPtr,
      author,
      isCustomLineOnly,
      isEmpty,
      isStartOfGroup,
      isEndOfGroup: false, // fill later
      isNewDate,
      isEdited,
      isEditing,
      isReplyingTo,
      isSelected,
      isOverflowing,
      replyTo: null, // fill later
    };
    views.push(richMessage);
    viewsById[message.id] = richMessage;
  }
  // replyTo
  for (const view of views) {
    if (view.message.replyToPtr != null) {
      const replyTo = viewsById[view.message.replyToPtr.id!];
      if (replyTo != null) {
        view.replyTo = replyTo;
        view.isStartOfGroup = true; // always begin new group for reply
      }
    }
  }
  // isEndOfGroup
  for (let i = 1; i < views.length; i++) {
    if (views[i].isStartOfGroup) {
      views[i - 1].isEndOfGroup = true;
    }
  }
  if (views.length > 0) {
    views[views.length - 1].isEndOfGroup = true;
  }
  return views;
});
const replyTo = computed(() => {
  if (draftReplyTo.value == null) return null;
  return messageViews.value.find((m) => m.message.id == draftReplyTo.value?.id);
});

// draft
const draftText: Ref<TextData | null> = ref(null);
const draftNodesPtr: Ref<NodeReferenceData[]> = ref([]);
const draftReplyTo: Ref<NodeReferenceData | null> = ref(null);
const draftNodes = supergraph.getManyRef(draftNodesPtr);

//
// Interaction
//

const inputContainerRef = ref<HTMLInputElement | null>(null);
const inputRef = ref<InstanceType<typeof Text> | null>(null);
const inputContainerSize = useElementSize(inputContainerRef);
const containerRef = ref<HTMLDivElement | null>(null);
const bodyScrollRef = ref<InstanceType<typeof Scroll> | null>(null);
const innerScrollRef = ref<HTMLElement | null>(null);
const editingTextRefs = ref<InstanceType<typeof Text>[] | null>(null); // there can only be one but it's inside a v-for (so it has to be an array)
const bodyHeight = computed(() => {
  return props.size?.height != null
    ? props.size.height - inputContainerSize.height.value - (props.isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0)
    : undefined;
});
const contextPopoverRef = ref<InstanceType<typeof Popover> | null>(null);

// selection
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: innerScrollRef, overlayEl: selectionOverlayRef });

// automatically stick to end (whenever we're at the end)
const stickToEnd: Ref<boolean> = ref(true);
watchEffect(() => {
  if (bodyScrollRef.value?.isScrolling) {
    stickToEnd.value = false;
  } else if (bodyScrollRef.value?.isAtEnd) {
    stickToEnd.value = true;
  }
});
watch(messages, (newMessages, oldMessages) => {
  if (oldMessages.length > 0 && isConnected.value && bodyScrollRef.value?.isCloseToEnd) {
    stickToEnd.value = true;
    nextTick(() => {
      bodyScrollRef.value?.scrollToEnd();
    });
  }
});

// editing

const currentAuthor = user;
const editingText = ref<TextData | null>(null);
const editingPtr = ref<NodeReferenceData | null>(null);

function startEdit(message: MessageData) {
  if (draftReplyTo.value != null) {
    stopReplying();
  }
  editingPtr.value = toNodeRef(message);
  editingText.value = message.text ?? null;
  nextTick(() => {
    editingTextRefs.value?.[0]?.focus?.();
  });
}

function stopEditing() {
  editingPtr.value = null;
  editingText.value = null;
}

function submitEdit() {
  const message = messages.value.find((m) => m.id == editingPtr.value?.id);
  if (message == null) throw new Error("message not found");
  const text = trimText(editingText.value ?? emptyText());
  if (isTextEmpty(text) && draftNodesPtr.value.length == 0) return; // don't create empty messages
  const { connection } = supergraph.getLinkOrError(toNodeRef(message));
  connection.tx.update(message, { text, editedAt: Timestamp.now() });
  stopEditing();
}

// draft

function submit() {
  const text = trimText(draftText.value ?? emptyText());
  if (isTextEmpty(text) && draftNodesPtr?.value?.length == 0) return; // don't create empty messages
  if (benchPtr.value == null) throw new Error("no bench");
  if (space.value == null) throw new Error("no space");
  if (currentAuthor.value == null) throw new Error("no current author");

  let tx = isEnabled.value ? txFactory() : benchConnection.tx;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Submit" } });
  }
  let thread: ThreadData;
  if (nodePtr.value == null) {
    thread = createThread(tx, benchGraph, {
      thread: {
        parentPtr: toNodeRef(pkg.value!),
        packagePtr: toNodeRef(pkg.value!),
      },
      members: [BENCH_BENCH_AGENT_PTR, "user"],
    });
    tx.update(space.value, { threadPtr: toNodeRef(thread) }); // not sure if this is right?
  } else {
    thread = supergraph.getOrError(nodePtr.value) as ThreadData;
  }

  // create message
  createMessage(tx, benchGraph, {
    message: {
      type: MessageType.DEFAULT,
      parentPtr: toNodeRef(thread),
      ownedByPtr: toNodeRef(currentAuthor.value),
      benchPtr: benchPtr.value,
      packagePtr: packagePtr.value!,
      threadPtr: toNodeRef(thread),
      replyToPtr: replyTo.value != null ? (draftReplyTo.value ?? undefined) : undefined,
      nodesPtr: draftNodesPtr.value,
      text,
    },
  });
  touchProcess(tx, thread);
  stickToEnd.value = true;
  nextTick(() => {
    bodyScrollRef.value?.scrollToEnd();
  });
}

function clearDraft() {
  draftText.value = null;
  draftNodesPtr.value = [];
  draftReplyTo.value = null;
}

function startReplying(message: MessageData) {
  draftReplyTo.value = toNodeRef(message);
  nextTick(() => {
    inputRef.value?.focus?.();
  });
}

function stopReplying() {
  draftReplyTo.value = null;
}

function addNodes(nodePtrs: NodeReferenceData[]) {
  if (thread.value == null) {
    throw new Error("no thread");
  }
  draftNodesPtr.value = [
    ...(draftNodesPtr.value ?? []).filter((n) => !nodePtrs.some((n2) => n2.id == n.id)),
    ...nodePtrs,
  ];
  // auto-add any pages to our claims (if not there already)
  for (const nodePtr of nodePtrs) {
    if (nodePtr.nodeType == NodeType.PAGE && !claims.value.some((c) => c.targetPtr?.id == nodePtr.id)) {
      const claim = createClaim(threadConnection.tx, threadGraph, {
        claim: {
          type: ClaimType.WRITE,
          parentPtr: toNodeRef(thread.value),
          benchPtr: thread.value?.benchPtr,
          packagePtr: thread.value?.packagePtr,
          targetPtr: nodePtr,
        },
      });
      claims.value.push(claim);
    }
  }
}

function removeNodes(nodePtrs: (NodeReferenceData | AnyNodeData)[]) {
  draftNodesPtr.value = draftNodesPtr.value?.filter((n) => !nodePtrs.some((n2) => n2.id == n.id));
}

async function addFiles(files: FileList | File[]) {
  Array.from(files).forEach(async (file) => {
    // upload and insert each file individually
    if (bench.value == null) throw new Error("no bench");
    if (pkg.value == null) throw new Error("no package");
    const thread = nodePtr.value != null ? (supergraph.get(nodePtr.value) as ThreadData | null) : null;
    const upload = uploadFile(() => benchConnection.tx, file, {
      bench: bench.value,
      pkg: pkg.value,
      parent: thread ?? pkg.value,
      compress: true,
    });
    await upload.completion.wait();
    addNodes([toNodeRef(upload.file.value!)]);
  });
}

// drop
const dropZone = useSingleDropZone({
  container: containerRef,
  orientation: Orientation.VERTICAL,
  name: "chat",
  kinds: ["file", "node", "selection"],
  onDrop: (dragged, anchor, event) => {
    if (dragged.kind == "file") {
      if (dragged.files == null) return;
      addFiles(dragged.files);
    } else if (dragged.kind == "node") {
      const nodePtr = toNodeRef(dragged.node);
      addNodes([nodePtr]);
    } else if (dragged.kind == "selection") {
      addNodes(dragged.nodes.map((n) => toNodeRef(n)));
    }
  },
});

// clipboard
useEventListener(inputContainerRef, "paste", (event) => {
  if (event.clipboardData == null) return;
  const files = Array.from(event.clipboardData.files);
  if (files.length == 0) return;
  addFiles(files);
});

// commands
const commands: Partial<CommandMapKit<"chat">> = {
  "chat.message.copy": {
    command: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      // just copy the text?
      const textParts: string[] = [];
      for (const message of messages) {
        const text = message.text;
        if (text != null) {
          textParts.push(renderText(text, null, "\n") ?? "");
        }
      }
      navigator.clipboard.writeText(textParts.join("\n"));
    },
  },
  "chat.message.reply": {
    command: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      startReplying(messages[0]);
    },
  },
  "chat.message.edit": {
    isEnabled: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      return messages.length == 1 && messages.every((m) => m.createdByPtr?.id == currentAuthor.value?.id);
    },
    command: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      startEdit(messages[0]);
    },
  },
};

function focus() {
  inputRef.value?.focus?.();
}

// view
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

defineExpose<ViewExpose>({ self, id, commands, focus });
</script>
<template>
  <div ref="containerRef">
    <!-- Root header -->
    <!-- NOTE :Incomplete: call/videochat with Thread members? (would be pretty cool) -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus-ptr="focusPtr" :graph="graph" />

    <!-- Drop overlay -->
    <div
      v-if="dropZone.activeDropZone.value"
      class="pointer-events-none absolute inset-0 z-100 flex h-full w-full items-center justify-center bg-gray-700/20"
    >
      <!-- Drop  -->
      <div
        class="flex flex-col items-center justify-center gap-y-1 rounded-md border border-gray-400 bg-white px-6 py-3"
      >
        <!-- Icons -->
        <div class="flex flex-row">
          <i
            v-for="icon in ['fas fa-file-word -rotate-12', 'fas fa-file-image', 'fas fa-file-vector rotate-12']"
            :key="icon"
            :class="[icon, 'rounded-sm bg-white text-3xl text-gray-700']"
          />
        </div>
        <!-- Text -->
        <div class="flex flex-col items-center justify-center gap-x-1 text-gray-900">
          <h3 class="text-base font-semibold">Add Anything</h3>
        </div>
      </div>
    </div>

    <!-- TODO :UX: autoscroll/load Chat more smoothly (sometimes it jumps, sometimes it doesn't stick to the bottom, ...) -->
    <!-- also see https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-anchor/Guide_to_scroll_anchoring -->

    <!-- Body -->
    <Scroll
      id="scroll"
      ref="bodyScrollRef"
      class="relative"
      :orientation="Orientation.VERTICAL"
      :stick-to-end="stickToEnd"
      :size="{ height: bodyHeight }"
    >
      <ul
        ref="innerScrollRef"
        class="relative mx-auto flex flex-col focus:outline-hidden"
        :class="[alignment == Alignment.END ? 'justify-end' : '']"
        :style="{
          minHeight: bodyHeight != null ? bodyHeight - 4 /* WHY -4? */ + 'px' : undefined,
          maxWidth: MAX_THREAD_WIDTH + 'px',
        }"
        @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
      >
        <!-- Top placeholder / general loading state -->
        <Transition
          appear
          enter-active-class="transition-opacity duration-300"
          leave-active-class="transition-opacity duration-75"
          enter-from-class="opacity-0"
          enter-to-class="opacity-100"
          leave-from-class="opacity-100"
          leave-to-class="opacity-0"
        >
          <div v-if="isEnabled && (!isAtStart || thread == null)">
            <div
              v-for="i in LOADING_SKELETON_COUNT"
              ref="topPlaceholderRef"
              :key="i"
              class="mx-auto mt-3 mb-2 flex animate-pulse flex-row"
              :style="{ width: widths.body + 'px' }"
            >
              <div :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" class="flex flex-col items-center">
                <div class="h-8 w-8 rounded-full bg-gray-100"></div>
              </div>
              <div class="flex flex-1 flex-col">
                <div class="mb-1.5 h-2 w-20 rounded-sm bg-gray-100" />
                <div v-for="j in Math.max(1, i % 3)" :key="j" class="my-[3px] h-[20px] rounded-sm bg-gray-100" />
              </div>
            </div>
          </div>
        </Transition>

        <!-- Beginning of Chat -->
        <div v-if="isAtStart && !isMinimal" class="mx-auto mb-3" :style="{ width: widths.body + 'px' }">
          <!-- Title -->
          <NodeReference
            v-if="thread"
            ref="nameRef"
            class="group/title mt-3 w-full px-0.5"
            :orientation="Orientation.VERTICAL"
            size="title"
            :node="thread"
            is-input
            hide-metadata
            :tx="() => connection!.tx"
            @navigate="(direction) => emit('navigate', direction)"
          >
            <!-- Meta -->
            <template #right>
              <!-- Open in full -->
              <button
                v-if="!isRoot"
                v-tooltip="{ small: true, text: 'Open in full' }"
                class="ml-2 cursor-pointer rounded-sm px-1 text-2xl text-gray-400 opacity-0 transition-opacity duration-75 group-focus-within/title:opacity-100 group-hover/title:opacity-100 hover:bg-gray-100 hover:text-gray-700"
                @click="() => canvas.goToNode(thread!)"
              >
                <i class="fas fa-arrow-up-right" />
              </button>
            </template>
          </NodeReference>
          <!-- Beginning -->
          <div v-if="thread" class="mt-1.5 px-0.5 text-base text-gray-400">
            <span>
              This is the beginning of this
              {{ nodePtr != null ? toCamelName(NodeType, nodePtr.nodeType) : "???" }}.
            </span>
          </div>
        </div>

        <!-- Message -->
        <template
          v-for="{
            idx,
            message,
            nodesPtr,
            author,
            replyTo,
            isEmpty,
            isEdited,
            isCustomLineOnly,
            isStartOfGroup,
            isEndOfGroup,
            isNewDate,
            isEditing,
            isSelected,
            isReplyingTo,
            isOverflowing,
          } in messageViews"
          v-if="thread != null"
          :key="message.id"
        >
          <!-- New date (line with date in middle) -->
          <div v-if="isNewDate" class="relative mx-auto mb-2 flex items-center" :style="{ width: widths.body + 'px' }">
            <div class="grow border-t border-gray-200" />
            <div class="mx-4 shrink text-sm text-gray-400">
              {{ formatAbsoluteDate(message.createdAt!, { prefer: "date" }) }}
            </div>
            <div class="grow border-t border-gray-200"></div>
          </div>

          <!-- Message -->
          <li
            class="group/message mx-auto rounded-sm px-0.5 transition-colors duration-75"
            :class="[
              isStartOfGroup ? 'mt-0.5 pt-0.5' : 'rounded-t-none',
              isEndOfGroup ? 'mb-0.5 pb-0.5' : 'rounded-b-none',
              isSelected ? 'bg-amber-100' : '',
              isReplyingTo ? 'bg-gray-100' : '',
            ]"
            :style="{
              width: widths.body + 'px',
            }"
            :data-node-type="message.metatype"
            :data-node-id="message.id"
            :data-node-ck="(message as any).ck"
            :data-node-bench-id="(message as any).benchPtr?.id"
            data-contextmenu-items="chat.message*"
          >
            <!-- Replying to -->
            <div v-if="replyTo != null" class="relative flex max-w-full items-center">
              <!-- 'Line' (supposed to go from avatar to the author with a bend) -->
              <div
                class="absolute top-2 h-4 w-7 rounded-sm rounded-r-none rounded-b-none border-t-2 border-l-2"
                :style="{ left: MESSAGE_SIDE_WIDTH / 2 - 3 + 'px' }"
              />
              <!-- Spacing for side -->
              <div class="" :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" />
              <!-- Author -->
              <span class="shrink-0 text-gray-700">@{{ replyTo.author?.name ?? "???" }}</span>
              <!-- Preview -->
              <span
                v-if="replyTo.message.text"
                class="ml-1 min-w-0 flex-1 truncate text-xs text-gray-400"
                :style="{ maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px' }"
              >
                {{ renderText(replyTo.message.text, 3) }}
              </span>
            </div>

            <!-- Body -->
            <div class="relative flex flex-row rounded-sm transition-colors duration-75">
              <!-- Commands -->
              <div
                v-if="!isEditing"
                class="absolute top-0 -right-2 z-10 flex -translate-y-[80%] flex-row rounded-lg border border-gray-200 bg-white opacity-0 transition-opacity duration-75 group-hover/message:opacity-100"
              >
                <button
                  v-for="command in MESSAGE_CONTEXT_COMMANDS.map(getCommand)"
                  :key="command.id"
                  v-tooltip="{ small: true, title: command.title, group: 'message' }"
                  class="cursor-pointer rounded-sm px-1.5 py-0.5 text-gray-400 transition-colors duration-75 enabled:hover:bg-gray-100 enabled:hover:text-gray-700"
                  :disabled="command.id == 'chat.message.edit' && message.createdByPtr?.id != currentAuthor?.id"
                  @click.stop.prevent="fireCommand(command, { nodes: [message] })"
                >
                  <IconInline v-bind="command.icon" />
                </button>
              </div>

              <!-- Side -->
              <div
                class="shrink-0 text-center"
                :class="[isStartOfGroup ? 'mt-1' : 'mt-0.5']"
                :style="{
                  width: MESSAGE_SIDE_WIDTH + 'px',
                }"
              >
                <!-- Icon -->
                <div
                  v-if="isCustomLineOnly"
                  class="ml-2 flex h-7 w-8 flex-col items-center justify-center text-gray-700"
                >
                  <IconInline v-bind="makeIcon(MessageTypeOptionInfo[message.type]?.icon ?? 'fas fa-question')" />
                </div>
                <AvatarInline
                  v-else-if="isStartOfGroup && author?.icon"
                  class="mr-1 text-gray-700"
                  size="medium"
                  v-bind="author.icon"
                  :force-color="author.color ?? undefined"
                />
                <div v-else-if="isStartOfGroup" class="ml-2 h-8 w-8 rounded-full bg-gray-100" />
                <!-- Time/edited otherwise -->
                <div v-else class="pt-[4px] text-xs text-gray-400">
                  <!-- Time -->
                  <span class="hidden group-hover/message:inline">
                    {{ tsToDt(message.createdAt!).toLocaleString(DateTime.TIME_SIMPLE) }}
                  </span>
                  <!-- Edited? -->
                  <span v-if="isEdited" class="fas fa-pencil text-xs text-gray-300 group-hover/message:hidden" />
                </div>
              </div>

              <!-- Body -->
              <div class="relative flex-1">
                <!-- Custom line -->
                <div v-if="isCustomLineOnly" class="mt-1 mb-1 flex flex-row items-baseline gap-x-1.5">
                  <!-- Node -->
                  <NodeReference
                    v-for="nodePtr in message.nodesPtr"
                    :key="nodePtr.id"
                    :node-ptr="nodePtr"
                    size="sm"
                    hide-metadata
                    class="rounded-full py-0.5"
                  />
                  <!-- Verb -->
                  <span v-if="message.type == MessageType.JOIN" class="text-gray-400">joined.</span>
                  <span v-else-if="message.type == MessageType.LEAVE" class="text-gray-400">left.</span>
                  <span v-else-if="message.type == MessageType.RESOURCE" class="text-gray-400"
                    >{{ VERB_BY_RESOURCE_STATUS[message.resourceStatus ?? ResourceStatus.UNSPECIFIED] }}.</span
                  >
                  <!-- Timestamp -->
                  <span class="text-xs text-gray-400">
                    {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                  </span>
                </div>

                <!-- Header -->
                <div v-else-if="isStartOfGroup" class="">
                  <!-- Author -->
                  <span class="text-base font-medium decoration-gray-300 underline-offset-3 hover:underline">
                    {{ author?.name ?? "[missing]" }}
                  </span>
                  <!-- Timestamp -->
                  <span class="ml-1.5 text-xs text-gray-400">
                    {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                  </span>
                  <!-- Model name -->
                  <span v-if="message?.modelName" class="ml-1.5 text-xs text-gray-400">
                    {{ message?.modelName }}
                  </span>
                  <!-- Edited? -->
                  <span v-if="isEdited" class="fas fa-pencil ml-1 text-xs text-gray-300" />
                </div>

                <!-- Text -->
                <div v-if="!isEditing && (!isEmpty || message.nodesPtr.length == 0)" class="relative">
                  <Text
                    :id="'text-' + message.id"
                    placeholder="Empty message"
                    class="-mt-[2px]"
                    :style="{
                      overflow: isOverflowing ? 'hidden' : undefined,
                      maxHeight: isOverflowing ? MESSAGE_MAX_LINES + 'em' : undefined,
                      width: widths.text + 'px',
                    }"
                    is-minimal
                    :model-value="message.text"
                  />
                  <!-- Expand button -->
                  <button
                    v-if="isOverflowing"
                    class="group/expand relative mt-1.5 flex w-full cursor-pointer items-center rounded-sm py-0.5 transition-colors duration-75"
                    @click="expandedMessageIds.push(message.id)"
                  >
                    <div
                      class="flex-grow border-t border-gray-200 transition-colors duration-75 group-hover/expand:border-gray-300"
                    />
                    <div
                      class="mx-4 shrink text-xs text-gray-400 transition-colors duration-75 group-hover/expand:text-gray-500"
                    >
                      <span class="mr-1">Expand</span>
                      <i class="fas fa-chevron-down" />
                    </div>
                    <div
                      class="flex-grow border-t border-gray-200 transition-colors duration-75 group-hover/expand:border-gray-300"
                    />
                  </button>
                </div>
                <!-- Editing content -->
                <div v-else-if="isEditing" class="my-1">
                  <Text
                    :id="'text-' + message.id"
                    ref="editingTextRefs"
                    :model-value="editingText ?? undefined"
                    is-input
                    suppress-enter
                    suppress-drop
                    placeholder="Empty message"
                    @update:model-value="
                      (value) => {
                        editingText = value;
                      }
                    "
                    @keydown.enter="
                      (e: KeyboardEvent) => {
                        if (!e.shiftKey) {
                          submitEdit();
                          stopEditing();
                        }
                      }
                    "
                    @keydown.esc.stop.prevent="stopEditing()"
                  />
                  <div class="mt-0.5 flex-row text-xs text-gray-400">
                    <span>
                      escape to
                      <a href="#" class="text-amber-700 underline-offset-2 hover:underline" @click.stop="stopEditing()"
                        >cancel</a
                      >
                    </span>
                    •
                    <span>
                      enter to
                      <a href="#" class="text-amber-700 underline-offset-2 hover:underline" @click.stop="submitEdit()"
                        >save</a
                      >
                    </span>
                  </div>
                </div>

                <!-- Extras -->
                <!-- NOTE :UX: this should be a proper :FileGallery -->
                <div
                  v-if="!isCustomLineOnly && nodesPtr.length > 0"
                  class="mt-1.5 mb-2 flex flex-row flex-wrap items-start gap-x-2 gap-y-2"
                >
                  <template v-for="nodePtr in nodesPtr" :key="nodePtr.id">
                    <File
                      v-if="nodePtr.nodeType == NodeType.FILE"
                      :id="'file-' + nodePtr.id"
                      is-inline
                      :model-value="nodePtr"
                      :size="{
                        height: size?.height != null ? Math.min(300, size.height / 2) : undefined,
                      }"
                      class=""
                    />
                    <Run
                      v-else-if="nodePtr.nodeType == NodeType.RUN && graph != null"
                      :graph="graph"
                      :node-ptr="nodePtr"
                    />
                    <Link
                      v-else-if="nodePtr.nodeType == NodeType.LINK && graph != null"
                      :graph="graph"
                      :node-ptr="nodePtr"
                    />
                    <NodeReference
                      v-else
                      :node-ptr="nodePtr"
                      size="sm"
                      class="rounded-full border border-gray-200 px-2 py-0.5"
                    />
                  </template>
                </div>
              </div>
            </div>
          </li>
        </template>

        <!-- Loading down 'skeleton' -->
        <div
          v-for="i in LOADING_SKELETON_COUNT"
          v-if="!isAtEnd && isEnabled"
          ref="bottomPlaceholderRef"
          :key="i"
          class="mx-5 mt-3 mb-2 flex animate-pulse flex-row"
        >
          <div :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" class="flex flex-col items-center">
            <div class="h-8 w-8 rounded-full bg-gray-100"></div>
          </div>
          <div class="flex flex-1 flex-col">
            <div class="mb-1.5 h-2 w-20 rounded-sm bg-gray-100" />
            <div v-for="j in Math.max(1, i % 3)" :key="j" class="my-[3px] h-[20px] rounded-sm bg-gray-100" />
          </div>
        </div>

        <!-- Selection overlay -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </ul>
    </Scroll>

    <!-- Input -->
    <div
      ref="inputContainerRef"
      class="mx-auto w-full"
      :style="{
        width: widths.body + 'px',
      }"
      @mousedown="inputRef?.focus?.('right')"
    >
      <!-- Activity -->
      <div class="flex h-[20px] w-full flex-row items-center px-2.5 text-xs">
        <template v-if="activeAuthors.length > 0">
          <!-- Status icon -->
          <span class="fas fa-circle-small relative mr-1.5 text-blue-500">
            <span class="fas fa-circle-small absolute inset-0 animate-ping text-blue-500" />
          </span>
          <template v-for="(author, i) in activeAuthors" :key="author.node.id">
            <!-- Names -->
            <div
              class="cursor-pointer rounded-full decoration-gray-300 underline-offset-3 hover:cursor-pointer hover:underline"
              role="link"
              :class="i > 0 ? 'ml-1' : ''"
              @click="author && canvas.goToNode(author.node)"
            >
              <span class="font-medium text-gray-900">{{ author.name }}</span>
            </div>
            <span v-if="i < activeAuthors.length - 1">, </span>
          </template>
        </template>
      </div>

      <!-- Replying to -->
      <div
        v-if="replyTo != null"
        class="flex w-full flex-row items-baseline rounded-sm rounded-b-none border border-b-0 border-gray-200 bg-gray-100 px-2.5 py-1"
        role="button"
      >
        <span class="text-gray-700">Replying to</span>
        <span class="ml-1 font-medium text-gray-900">{{ replyTo.author?.name ?? "???" }}</span>
        <!-- Preview -->
        <span
          v-if="replyTo.message.text"
          class="ml-1.5 truncate text-xs text-gray-400"
          :style="{
            maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px',
          }"
        >
          {{ renderText(replyTo.message.text, 3) }}
        </span>
        <!-- Clear -->
        <button
          class="ml-auto cursor-pointer rounded-full px-1 text-gray-700 hover:text-gray-900"
          @click="stopReplying()"
        >
          <i class="fas fa-xmark" />
        </button>
      </div>

      <!-- Input -->
      <div
        class="relative flex flex-row rounded-sm border border-gray-200 px-2 pt-2 pb-2"
        :class="[replyTo != null ? 'rounded-t-none' : '']"
      >
        <!-- Left -->
        <div class="" :style="{ width: MESSAGE_SIDE_WIDTH - 9 + 'px' }">
          <!-- Add extra -->
          <Popover ref="contextPopoverRef" placement="top" :reference-margin="10">
            <template #trigger="{ toggle }">
              <button
                class="transition-color mr-3 w-[28px] cursor-pointer rounded-2xl border border-gray-200 bg-gray-100 px-1.5 py-0.5 text-gray-700 duration-75 hover:bg-gray-200"
                @mousedown.stop="toggle"
              >
                <i class="fas fa-plus" />
              </button>
            </template>
            <template #content="{ close }">
              <div
                v-outside.mousedown.stop="close"
                class="flex h-[200px] w-[300px] flex-col rounded-md border border-gray-200 bg-white p-2 shadow-md"
              >
                <!-- Header -->
                <div class="mb-2 flex flex-row items-center gap-x-0.5 border-b border-gray-200 pb-2">
                  <!-- Tabs -->
                  <button
                    v-for="tab in CONTEXT_TABS"
                    :key="tab"
                    class="cursor-pointer rounded-sm px-1.5 transition-colors duration-75 hover:bg-gray-100"
                    :class="[contextTab == tab ? 'bg-gray-100 text-gray-900' : 'text-gray-400']"
                    @mousedown.stop="contextTab = tab"
                  >
                    {{ tab }}
                  </button>
                  <!-- Close -->
                  <button
                    class="ml-auto cursor-pointer rounded-full px-1 text-gray-400 hover:text-gray-700"
                    @mousedown.stop="close"
                  >
                    <i class="fas fa-xmark w-5 text-center" />
                  </button>
                </div>

                <!-- Content -->
                <div class="max-h-full flex-1 overflow-y-scroll">
                  <File
                    v-if="contextTab == 'File'"
                    id="file"
                    class="h-full w-full"
                    is-inline
                    is-input
                    @update:model-value="
                      (filePtr) => {
                        if (filePtr != null) {
                          addNodes([filePtr]);
                        }
                      }
                    "
                  />
                  <ul v-else-if="contextTab == 'Pages'">
                    <li
                      v-for="claim in claims.filter((c) => c.targetPtr?.nodeType == NodeType.PAGE)"
                      :key="claim.id"
                      class="flex flex-row"
                    >
                      <NodeReference :node-ptr="claim.targetPtr" size="sm" class="rounded-full px-2 py-0.5" />
                      <button
                        class="ml-auto cursor-pointer rounded-full px-1 text-gray-400 hover:text-gray-700"
                        @mousedown.stop="
                          () => {
                            threadConnection.tx.delete(claim);
                          }
                        "
                      >
                        <i class="fas fa-minus w-5 text-center" />
                      </button>
                    </li>
                  </ul>
                </div>
              </div>
            </template>
          </Popover>
        </div>

        <!-- Input -->
        <div class="flex-1">
          <!-- Extras -->
          <div v-if="draftNodes.length > 0" class="mb-1 flex flex-row flex-wrap items-start gap-x-2.5 gap-y-1">
            <!-- should also be a proper :FileGallery -->
            <div v-for="node in draftNodes" :key="node.id" class="group/file relative">
              <File
                v-if="isNode(node, NodeType.FILE)"
                :id="'file-' + node.id"
                is-inline
                class="pointer-events-none"
                :model-value="toNodeRef(node)"
                :size="{
                  height:
                    size?.height != null && INLINABLE_FILE_TYPES.includes(node.type)
                      ? Math.min(100, size.height / 3)
                      : undefined,
                }"
              />
              <NodeReference v-else :node="node" size="sm" class="rounded-full border border-gray-200 px-2 py-0.5" />
              <!-- Remove -->
              <button
                class="absolute top-0 right-0 translate-x-1/2 -translate-y-1/2 rounded-full border border-gray-200 bg-gray-500 px-1 text-xs text-white transition-colors duration-75 hover:bg-gray-600"
                @click.stop="removeNodes([node])"
              >
                <i class="fas fa-xmark" />
              </button>
            </div>
          </div>
          <!-- Text -->
          <Scroll
            id="input-scroll"
            ref="inputScrollRef"
            :orientation="Orientation.VERTICAL"
            size-is-dynamic
            :size="{ height: size?.height != null ? size.height / 2 : undefined }"
          >
            <Text
              id="input"
              ref="inputRef"
              is-input
              is-minimal
              placeholder="Message..."
              suppress-enter
              suppress-drop
              :model-value="draftText!"
              @update:model-value="
                (value) => {
                  draftText = value;
                }
              "
              @keydown.enter="
                (e: KeyboardEvent) => {
                  if (!e.shiftKey) {
                    e.preventDefault();
                    submit();
                    clearDraft();
                  }
                }
              "
            />
          </Scroll>
        </div>

        <!-- Controls? (pause/stop/resume) -->
        <div class="mr-1">
          <!-- Record / Submit -->
          <button
            class="transition-color w-[28px] cursor-pointer rounded-2xl border border-gray-200 bg-gray-100 px-1.5 py-0.5 text-gray-700 duration-75 hover:bg-gray-200"
            @click="submit()"
          >
            <i class="fas fa-arrow-up" />
          </button>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex h-[28px] w-full flex-row flex-nowrap items-center gap-x-3">
        <!-- Context -->
        <div class="flex flex-row items-center gap-x-1.5">
          <!-- Computer -->
          <button
            :disabled="computer != null || hasClaimedComputer"
            class="group/button cursor-pointer rounded-full px-1 py-0.5 text-xs transition-colors duration-75 enabled:text-gray-400 enabled:hover:bg-gray-100 enabled:hover:text-gray-700"
            @mousedown.stop="
              () => {
                hasClaimedComputer = true;
                createClaim(threadConnection.tx, threadGraph, {
                  claim: {
                    mode: NodeMode.MAIN,
                    type: ClaimType.WRITE,
                    packagePtr: (thread as ThreadData)?.packagePtr,
                    parentPtr: nodePtr,
                    targetTemplatePtr: BENCH_BENCH_UBUNTU_DESKTOP_PTR,
                  },
                });
              }
            "
            @dblclick="
              () => {
                if (computer != null) {
                  canvas.goToNode(computer);
                }
              }
            "
          >
            <NodeReference v-if="computer != null" :node="computer" size="xs" is-light class="" />
            <template v-else>
              <IconInline v-bind="makeIcon('fa-solid fa-computer-classic')" class="w-5 text-center" />
              <span class="ml-1.5">Computer</span>
            </template>
          </button>
          <!-- Pages / Files / Links / ... -->
          <button
            v-if="claims.filter((c) => c.targetPtr?.nodeType == NodeType.PAGE).length > 0"
            class="cursor-pointer rounded-sm text-xs text-gray-400 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
            @mousedown.stop="
              () => {
                contextTab = 'Pages';
                contextPopoverRef?.open();
              }
            "
          >
            <span>{{ claims.filter((c) => c.targetPtr?.nodeType == NodeType.PAGE).length }} pages</span>
          </button>
        </div>

        <!-- Compute -->
        <div class="ml-auto">
          <Popover placement="top-left">
            <template #trigger="{ toggle }">
              <button
                class="flex cursor-pointer flex-row items-center rounded-full px-1 py-0.5 text-xs transition-colors duration-75 enabled:hover:bg-gray-100"
                :class="thread?.modelDeveloper == null ? 'text-gray-400' : 'text-gray-700'"
                @mousedown.stop="toggle"
              >
                <!-- Model type -->
                <template v-if="thread?.modelDeveloper == null">
                  <IconInline v-bind="makeIcon('fas fa-microchip')" class="w-5 text-center" />
                  <span class="ml-1">Model</span>
                </template>
                <template v-else>
                  <IconInline
                    v-bind="getEnumOption(EnumType.MODEL_DEVELOPER, thread?.modelDeveloper)?.icon"
                    class="w-5 text-center"
                  />
                  <span class="ml-1">{{ getEnumOption(EnumType.MODEL_DEVELOPER, thread?.modelDeveloper)?.title }}</span>
                </template>
                <!-- Juice/Effort? -->
              </button>
            </template>
            <template #content="{ close }">
              <div
                v-outside.mousedown.stop="close"
                class="pointer-events-auto z-70 w-40 rounded-sm border border-gray-200 bg-white text-gray-900 shadow-xs shadow-gray-300"
              >
                <!-- Model picker -->
                <ul class="mx-0.5 my-0.5 flex max-w-full flex-col py-0.5">
                  <!-- Auto -->
                  <li
                    class="mt-[1px] mr-0.5 mb-[1px] flex h-[30px] max-w-full cursor-pointer flex-row items-center truncate rounded-sm border border-transparent px-1.5 hover:bg-gray-100"
                    @click="
                      () => {
                        threadConnection.tx.update(thread!, { modelDeveloper: undefined });
                        close();
                      }
                    "
                  >
                    <span class="mr-1.5 w-6 shrink-0 text-gray-700"></span>
                    <span class="max-w-full truncate select-none">Automatic</span>
                  </li>
                  <!-- Options -->
                  <li
                    v-for="option in getEnumOptions(EnumType.MODEL_DEVELOPER)"
                    :key="option.value"
                    class="mt-[1px] mr-0.5 mb-[1px] flex h-[30px] max-w-full cursor-pointer flex-row items-center truncate rounded-sm border border-transparent px-1.5 hover:bg-gray-100"
                    @click="
                      () => {
                        threadConnection.tx.update(thread!, { modelDeveloper: option.value });
                        close();
                      }
                    "
                  >
                    <IconInline v-bind="option.icon" class="mr-1.5 w-6 shrink-0 text-center text-gray-700" />
                    <span class="max-w-full truncate select-none">
                      {{ option.title }}
                    </span>
                  </li>
                </ul>
              </div>
            </template>
          </Popover>
        </div>
      </div>
    </div>
  </div>
</template>
