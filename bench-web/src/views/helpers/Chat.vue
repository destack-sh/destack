<script lang="ts" setup>
import { supergraph } from "@/globals";
import { EditSubject, makeAndConditional, makeExpression } from "@/language/core/expression";
import { useSubnodeProperty } from "@/language/core/node";
import { emptyText, isTextEmpty, renderText, trimText } from "@/language/core/text";
import { INLINE_FILE_TYPES, uploadFile } from "@/language/resource/file";
import { newChangeId } from "@/language/runtime/transaction";
import { createThread } from "@/language/source/thread";
import { createMessage, getMessageAuthorPtr } from "@/language/state/message";
import {
  Alignment,
  AnyNodeData,
  ChannelData,
  ExpressionData,
  ExpressionType,
  FileData,
  IconData,
  MessageData,
  MessageProperty,
  MessageType,
  NodeReferenceData,
  NodeType,
  Orientation,
  RectangleData,
  TextData,
  ThreadData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE, benchPtr, packagePtr } from "@/system/client";
import { SearchConnectionParams, useInfiniteSearchConnection } from "@/system/connection";
import { bench, benchConnection, benchGraph, canvas, pkg, space } from "@/system/space";
import { user } from "@/system/user";
import { CommandMapKit, fireCommand, getCommand, getNodesForCommand, MESSAGE_CONTEXT_COMMANDS } from "@/ui/command";
import { useSingleDropZone } from "@/ui/drag";
import { AvatarInline, getNodeIcon, getNodeTitle, IconInline } from "@/ui/icon";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate, getNow, TimeUpdateInterval, tsToDt } from "@/utils/time";
import RootHeader from "@/views/builtins/RootHeader.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import Text from "@/views/content/Text.vue";
import { useElementSize, useElementVisibility, useEventListener } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, nextTick, Ref, ref, toRef, watch, watchEffect } from "vue";

const LOADING_SKELETON_COUNT = 3;
const CHUNK_SIZE = 80;
const MIN_AUTOSCROLL_INTERVAL_MILLISECONDS = 500;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MESSAGE_HEIGHT_MIN = 28;
const MESSAGE_MAX_TIME_DELTA_SECONDS = 5 * 60; // 5 minutes
const MESSAGE_SIDE_WIDTH = 52;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    isRoot?: boolean;
  } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focus" | "selection" | "alignment" | "subnodePacked">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

// node
const nodePtr = computedValue(() => props.nodePtr);
const node = supergraph.getRef(nodePtr) as Ref<AnyNodeData>;
const channelPtr = computed(() => {
  if (isNode(node.value, NodeType.CHANNEL)) {
    return nodePtr.value;
  } else if (isNode(node.value, NodeType.THREAD)) {
    return node.value.channelPtr!;
  }
  return null;
});
const threadPtr = computed(() => {
  if (isNode(node.value, NodeType.THREAD)) {
    return nodePtr.value;
  }
  return null;
});

//
// Messages
//

const isEnabled = computed(() => threadPtr.value != null);
const filter = computed(() => {
  const filters: ExpressionData[] = [];
  // channel
  if (channelPtr.value != null) {
    filters.push(
      makeExpression({
        type: ExpressionType.EQUALS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.channelPtr),
        value: channelPtr.value,
      }),
    );
  }
  // thread
  if (threadPtr.value != null) {
    filters.push(
      makeExpression({
        type: ExpressionType.EQUALS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.threadPtr),
        value: threadPtr.value,
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
      scope: BENCH_SCOPE.value,
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

const authorsPtr: Ref<NodeReferenceData[]> = computed(() => {
  const authorsPtrById: Record<string, NodeReferenceData> = {};
  for (const message of messages.value) {
    if (message.createdByPtr != null) {
      authorsPtrById[message.createdByPtr.id!] = message.createdByPtr;
    }
  }
  return Object.values(authorsPtrById);
});
const authors = supergraph.getManyRef(authorsPtr);
const authorsById: Ref<Record<string, EditSubject>> = computed(() =>
  authors.value.reduce(
    (acc, author) => {
      acc[author.id!] = author as EditSubject;
      return acc;
    },
    {} as Record<string, EditSubject>,
  ),
);
type MessageView = {
  idx: number;
  message: MessageData;
  filesPtr: NodeReferenceData[];
  author: EditSubject | null;
  authorIcon: IconData | null;
  authorName: string | null;
  replyTo: MessageView | null;
  isEmpty: boolean;
  isNewGroup: boolean;
  isNewDate: boolean;
  isEdited: boolean;
  isEditing: boolean;
  isReplyingTo: boolean;
  isSelected: boolean;
};
const messageViews = computed(() => {
  const views: MessageView[] = [];
  const viewsById: Record<string, MessageView> = {};
  for (let i = 0; i < messages.value.length; i++) {
    const message = messages.value[i];
    const filesPtr = message.nodesPtr.filter((n) => n.nodeType == NodeType.FILE);
    const authorPtr = getMessageAuthorPtr(message);
    const author = authorPtr != null ? (authorsById.value[authorPtr.id!] ?? supergraph.get(authorPtr)) : null;
    const authorIcon = author != null ? (getNodeIcon(author) ?? null) : null;
    const authorName = author != null ? (getNodeTitle(author) ?? null) : null;
    let isNewGroup;
    let isNewDate;
    if (i == 0) {
      isNewGroup = true;
      isNewDate = true;
    } else {
      const previousDt = tsToDt(messages.value[i - 1].createdAt!);
      const currentDt = tsToDt(message.createdAt!);
      isNewGroup =
        message.createdByPtr?.id != messages.value[i - 1]?.createdByPtr?.id ||
        Math.abs(Number(messages.value[i - 1].createdAt!.seconds) - Number(message.createdAt!.seconds)) >
          MESSAGE_MAX_TIME_DELTA_SECONDS;
      isNewDate = previousDt.day != currentDt.day;
    }
    const isEmpty = message.text == null || isTextEmpty(message.text);
    const isEdited = message.updatedAt?.seconds != message.createdAt?.seconds;
    const isEditing = editingPtr.value?.id == message.id;
    const isReplyingTo = draftReplyTo.value?.id == message.id;
    const isSelected = canvas.isSelected(message);
    const richMessage: MessageView = {
      idx: i,
      message,
      filesPtr,
      author,
      authorIcon,
      authorName,
      isEmpty,
      isNewGroup,
      isNewDate,
      isEdited,
      isEditing,
      isReplyingTo,
      isSelected,
      replyTo: null, // fill later
    };
    views.push(richMessage);
    viewsById[message.id] = richMessage;
  }
  // fill in replyTo
  for (const view of views) {
    if (view.message.replyToPtr != null) {
      const replyTo = viewsById[view.message.replyToPtr.id!];
      if (replyTo != null) {
        view.replyTo = replyTo;
        view.isNewGroup = true; // always begin new group for reply
      }
    }
  }
  return views;
});
const replyTo = computed(() => {
  if (draftReplyTo.value == null) return null;
  return messageViews.value.find((m) => m.message.id == draftReplyTo.value?.id);
});

// draft
const draftText = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "draftText");
const draftNodesPtr = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "draftNodesPtr");
const draftReplyTo = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "draftReplyToPtr");
const draftNodes = supergraph.getManyRef(draftNodesPtr);
const draftFiles = computed(() => draftNodes.value.filter((n) => isNode(n, NodeType.FILE)));

//
// Intercommand
//

const inputContainerRef = ref<HTMLInputElement | null>(null);
const inputRef = ref<InstanceType<typeof Text> | null>(null);
const inputContainerSize = useElementSize(inputContainerRef);
const containerRef = ref<HTMLDivElement | null>(null);
const bodyScrollRef = ref<InstanceType<typeof Scroll> | null>(null);
const editingTextRefs = ref<InstanceType<typeof Text>[] | null>(null); // there can only be one but it's inside a v-for (so it has to be an array)
const bodyHeight = computed(() => {
  return props.size?.height != null
    ? props.size.height - inputContainerSize.height.value - (props.isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0)
    : undefined;
});

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
  connection.tx.update(message, { text });
}

// draft

function submit() {
  const text = trimText(draftText.value ?? emptyText());
  if (isTextEmpty(text) && draftNodesPtr.value.length == 0) return; // don't create empty messages
  if (benchPtr.value == null) throw new Error("no bench");
  if (space.value == null) throw new Error("no space");

  let tx = isEnabled.value ? txFactory() : benchConnection.tx;
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Submit" } });
  }
  const messageChannelPtr = channelPtr.value ?? undefined;
  let messageThreadPtr = threadPtr.value ?? undefined;

  // create thread if needed
  if (messageThreadPtr == null) {
    if (messageChannelPtr == null) throw new Error("no channel");
    const thread = createThread(tx, benchGraph, {
      thread: {
        parentPtr: messageChannelPtr,
        packagePtr: packagePtr.value!,
      },
    });
    messageThreadPtr = toNodeRef(thread);
  }

  // create message
  createMessage(tx, benchGraph, {
    message: {
      type: replyTo.value != null ? MessageType.REPLY : MessageType.REGULAR,
      parentPtr: messageThreadPtr ?? messageChannelPtr,
      benchPtr: benchPtr.value,
      packagePtr: packagePtr.value!,
      channelPtr: messageChannelPtr,
      threadPtr: messageThreadPtr ?? undefined,
      replyToPtr: replyTo.value != null ? draftReplyTo.value : undefined,
      nodesPtr: draftNodesPtr.value,
      text,
    },
  });
  stickToEnd.value = true;
  bodyScrollRef.value?.scrollToEnd();
}

function clearDraft() {
  state.update(
    {
      metatype: NodeType.VIEW,
      type: ViewType.CHAT,
      subnode: { draftText: undefined, draftNodesPtr: [], draftReplyToPtr: undefined },
    },
    { debounce: "tick" },
  );
}

function startReplying(message: MessageData) {
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { draftReplyToPtr: toNodeRef(message) } },
    { debounce: "tick" },
  );
  nextTick(() => {
    inputRef.value?.focus?.();
  });
}

function stopReplying() {
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { draftReplyToPtr: undefined } },
    { debounce: "tick" },
  );
}

async function addFiles(files: FileList | File[]) {
  Array.from(files).forEach(async (file) => {
    // upload and insert each file individually
    if (bench.value == null) throw new Error("no bench");
    if (pkg.value == null) throw new Error("no package");
    const thread = threadPtr.value != null ? (supergraph.get(threadPtr.value) as ThreadData | null) : null;
    const channel = channelPtr.value != null ? (supergraph.get(channelPtr.value) as ChannelData | null) : null;
    const upload = uploadFile(() => benchConnection.tx, file, {
      bench: bench.value,
      pkg: pkg.value,
      parent: thread ?? channel ?? pkg.value,
    });
    await upload.completion.wait();
    state.update(
      {
        metatype: NodeType.VIEW,
        type: ViewType.CHAT,
        subnode: { draftNodesPtr: [...(draftNodesPtr.value ?? []), toNodeRef(upload.file.value!)] },
      },
      { debounce: "tick" },
    );
  });
}

function removeFiles(files: (FileData | NodeReferenceData)[]) {
  state.update(
    {
      metatype: NodeType.VIEW,
      type: ViewType.CHAT,
      subnode: { draftNodesPtr: draftNodesPtr.value?.filter((f) => !files.some((f2) => f2.id == f.id)) },
    },
    { debounce: "tick" },
  );
}

// drop
const dropZone = useSingleDropZone({
  container: containerRef,
  orientation: Orientation.VERTICAL,
  name: "chat",
  kinds: ["file"],
  onDrop: (dragged, anchor, event) => {
    if (dragged.kind == "file") {
      if (dragged.files == null) return;
      addFiles(dragged.files);
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
  "chat.message.reply": {
    command: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      startReplying(messages[0]);
    },
  },
  "chat.message.edit": {
    isEnabled: (command, ctx) => {
      const { nodes: messages } = getNodesForCommand(command, ctx, [NodeType.MESSAGE]);
      return messages.length > 0 && messages.every((m) => m.createdByPtr?.id == currentAuthor.value?.id);
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

defineExpose<ViewExpose>({ self, id, commands: commands, focus });
</script>
<template>
  <div ref="containerRef" class="relative">
    <!-- Root header -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="$props.focus" :graph="benchGraph" />

    <!-- Drop zone (overlay) -->
    <div
      v-if="dropZone.activeDropZone.value"
      class="pointer-events-none absolute z-40 flex h-full w-full items-center justify-center bg-gray-400/40"
    >
      <div class="flex flex-col items-center justify-center gap-y-1">
        <i class="fas fa-upload text-3xl text-gray-700/80" />
        <div class="flex flex-row items-center justify-center gap-x-1 text-base font-medium text-gray-700/80">
          <span>Upload Files</span>
        </div>
      </div>
    </div>

    <!-- TODO :UX: autoscroll Chat more smoothly (sometimes it jumps, especially when scrolling down) -->
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
      <!-- Messages -->
      <ul
        class="relative mb-4 mt-2 flex flex-col focus:outline-none"
        :class="[props.alignment == Alignment.END ? 'justify-end' : '']"
        :style="{ minHeight: bodyHeight != null ? bodyHeight - 32 + 'px' : undefined }"
      >
        <!-- Top placeholder -->
        <div
          v-for="i in LOADING_SKELETON_COUNT"
          v-if="!isAtStart && isEnabled"
          ref="topPlaceholderRef"
          :key="i"
          class="mx-5 mb-2 mt-3 flex animate-pulse flex-row"
        >
          <div :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" class="flex flex-col items-center">
            <div class="h-8 w-8 rounded-full bg-gray-100"></div>
          </div>
          <div class="flex flex-1 flex-col">
            <div class="mb-1.5 h-2 w-20 rounded bg-gray-100" />
            <div v-for="j in Math.max(1, i % 3)" :key="j" class="my-[3px] h-[20px] rounded bg-gray-100" />
          </div>
        </div>

        <!-- Empty Chat -->
        <div v-if="!isEnabled && messageViews.length == 0" class="mx-5">
          <slot name="empty" />
        </div>
        <!-- Beginning of Chat -->
        <div v-else-if="isAtStart && $slots.beginning != null" class="mx-5 mb-2">
          <slot name="beginning" />
        </div>

        <!-- Message -->
        <li
          v-for="{
            idx,
            message,
            filesPtr,
            author,
            replyTo,
            authorIcon,
            authorName,
            isEmpty,
            isEdited,
            isNewGroup,
            isNewDate,
            isEditing,
            isSelected,
            isReplyingTo,
          } in messageViews"
          :key="message.id"
          class="group/message mx-5 max-w-full"
          :class="[isNewGroup && idx != 0 ? 'mt-2.5' : '']"
          :data-node-id="message.id"
          :data-node-type="message.metatype"
          data-contextmenu-items="chat.message*"
          data-suppress-node="self"
          @dblclick="startReplying(message)"
        >
          <!-- New date (line with date in middle) -->
          <div v-if="isNewDate" class="relative mb-2 flex items-center">
            <div class="flex-grow border-t border-gray-200" />
            <div class="mx-4 flex-shrink text-sm text-gray-400">
              {{ formatAbsoluteDate(message.createdAt!, { prefer: "date" }) }}
            </div>
            <div class="flex-grow border-t border-gray-200"></div>
          </div>

          <!-- Replying to -->
          <div v-if="replyTo != null" class="relative flex max-w-full items-center">
            <!-- 'Line' (supposed to go from avatar to the author with a bend) -->
            <div
              class="absolute top-2 h-4 w-7 rounded rounded-b-none rounded-r-none border-l-2 border-t-2"
              :style="{ left: MESSAGE_SIDE_WIDTH / 2 - 3 + 'px' }"
            />
            <!-- Spacing for side -->
            <div class="" :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" />
            <!-- Author -->
            <span class="flex-shrink-0 text-gray-700">@{{ replyTo.authorName ?? "???" }}</span>
            <!-- Preview -->
            <span
              v-if="replyTo.message.text"
              class="ml-1 min-w-0 flex-1 truncate text-xs text-gray-400"
              :style="{ maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px' }"
            >
              {{ renderText(replyTo.message.text) }}
            </span>
          </div>

          <!-- Body -->
          <div
            class="flex flex-row rounded transition-colors duration-150"
            :class="[
              isSelected ? 'bg-orange-100' : '',
              messageViews[idx - 1]?.isSelected && !isNewGroup ? 'rounded-t-none' : '',
              messageViews[idx + 1]?.isSelected && !messageViews[idx + 1]?.isNewGroup ? 'rounded-b-none' : '',
              isReplyingTo ? 'bg-gray-100' : '',
            ]"
          >
            <!-- Side -->
            <div
              class="flex-shrink-0 text-center"
              :class="[isNewGroup ? 'mt-1' : 'mt-0.5']"
              :style="{
                width: MESSAGE_SIDE_WIDTH + 'px',
              }"
            >
              <!-- Author for new groups -->
              <AvatarInline
                v-if="isNewGroup && authorIcon"
                class="mr-1 cursor-pointer text-gray-700"
                size="medium"
                v-bind="authorIcon"
                @click="author && canvas.goToNode(author)"
              />
              <div v-else-if="isNewGroup" class="ml-2 h-8 w-8 rounded-full bg-gray-100" />
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
              <!-- Meta (if new group) -->
              <div v-if="isNewGroup" class="">
                <!-- Author -->
                <span
                  class="text-base font-medium decoration-gray-300 underline-offset-3 hover:cursor-pointer hover:underline"
                  @click="author && canvas.goToNode(author)"
                >
                  {{ authorName ?? "[deleted]" }}
                </span>
                <!-- Timestamp -->
                <span class="ml-1.5 text-xs text-gray-400">
                  {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                </span>
                <!-- Edited? -->
                <span v-if="isEdited" class="fas fa-pencil ml-1 text-xs text-gray-300" />
              </div>
              <!-- Commands -->
              <div
                v-if="!isEditing"
                class="absolute right-0 top-0 z-10 flex flex-row rounded border border-gray-200 bg-white opacity-0 group-hover/message:opacity-100"
              >
                <button
                  v-for="command in MESSAGE_CONTEXT_COMMANDS.map(getCommand)"
                  :key="command.id"
                  v-tooltip="{ small: true, title: command.title, group: 'message' }"
                  class="cursor-pointer rounded px-1.5 py-0.5 text-gray-400 transition-colors duration-150 enabled:hover:bg-gray-100 enabled:hover:text-gray-700"
                  :disabled="command.id == 'chat.message.edit' && message.createdByPtr?.id != currentAuthor?.id"
                  @click.stop.prevent="fireCommand(command, { nodes: [message] })"
                >
                  <IconInline v-bind="command.icon" />
                </button>
              </div>
              <!-- Content -->
              <Text
                v-if="!isEditing && (!isEmpty || message.nodesPtr.length == 0)"
                :id="'text-' + message.id"
                placeholder="Empty message"
                is-minimal
                :model-value="message.text"
              />
              <div v-if="isEditing" class="my-1">
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
                <div v-if="isEditing" class="mt-0.5 flex-row text-xs text-gray-400">
                  <span>
                    escape to
                    <a href="#" class="text-primary-700 underline-offset-2 hover:underline" @click.stop="stopEditing()"
                      >cancel</a
                    >
                  </span>
                  •
                  <span>
                    enter to
                    <a href="#" class="text-primary-700 underline-offset-2 hover:underline" @click.stop="submitEdit()"
                      >save</a
                    >
                  </span>
                </div>
              </div>
              <!-- Extras -->
              <!-- NOTE :UX: this should be a proper :FileGallery -->
              <div v-if="filesPtr.length > 0" class="mb-2 mt-1.5 flex flex-row flex-wrap gap-x-2 gap-y-2">
                <File
                  v-for="filePtr in filesPtr"
                  :id="'file-' + filePtr.id"
                  :key="filePtr.id"
                  is-inline
                  :model-value="toNodeRef(filePtr)"
                  :size="{
                    height: size?.height != null ? Math.min(300, size.height / 2) : undefined,
                  }"
                  class=""
                />
              </div>
            </div>
          </div>
        </li>

        <!-- Loading down 'skeleton' -->
        <div
          v-for="i in LOADING_SKELETON_COUNT"
          v-if="!isAtEnd && isEnabled"
          ref="bottomPlaceholderRef"
          :key="i"
          class="mx-5 mb-2 mt-3 flex animate-pulse flex-row"
        >
          <div :style="{ width: MESSAGE_SIDE_WIDTH + 'px' }" class="flex flex-col items-center">
            <div class="h-8 w-8 rounded-full bg-gray-100"></div>
          </div>
          <div class="flex flex-1 flex-col">
            <div class="mb-1.5 h-2 w-20 rounded bg-gray-100" />
            <div v-for="j in Math.max(1, i % 3)" :key="j" class="my-[3px] h-[20px] rounded bg-gray-100" />
          </div>
        </div>
      </ul>
    </Scroll>

    <!-- Input -->
    <div ref="inputContainerRef" class="mx-5" @mousedown="inputRef?.focus?.('right')">
      <!-- Replying to -->
      <div
        v-if="replyTo != null"
        class="flex w-full flex-row items-baseline rounded rounded-b-none border border-b-0 border-gray-200 bg-gray-100 px-2.5 py-1"
        role="button"
      >
        <span class="text-gray-700">Replying to</span>
        <span class="ml-1 font-medium text-gray-900">{{ replyTo.authorName ?? "???" }}</span>
        <!-- Preview -->
        <span
          v-if="replyTo.message.text"
          class="ml-1.5 truncate text-xs text-gray-400"
          :style="{
            maxWidth: (size?.width != null ? size.width - 300 : 100) + 'px',
          }"
        >
          {{ renderText(replyTo.message.text) }}
        </span>
        <!-- Clear -->
        <button class="ml-auto rounded-full px-1 text-gray-700 hover:text-gray-900" @click="stopReplying()">
          <i class="fas fa-xmark" />
        </button>
      </div>
      <!-- Box -->
      <div
        class="peer relative rounded border border-gray-200 px-2 py-2"
        :class="[replyTo != null ? 'rounded-t-none' : '']"
      >
        <div class="flex flex-row">
          <!-- Side -->
          <div class="sticky top-0 text-center" :style="{ width: MESSAGE_SIDE_WIDTH - 8 + 'px' }">
            <!-- Add extra -->
            <button
              v-tooltip="{ small: true, title: 'Add Context' }"
              class="transition-color mr-3 rounded-2xl border border-gray-200 bg-gray-100 px-1.5 py-0.5 text-gray-700 duration-150 hover:bg-gray-200"
              @click.stop
            >
              <i class="fas fa-plus" />
            </button>
          </div>
          <!-- Input -->
          <div class="flex-1">
            <Scroll
              id="input-scroll"
              ref="inputScrollRef"
              :orientation="Orientation.VERTICAL"
              size-is-dynamic
              :size="{ height: size?.height != null ? size.height / 2 : undefined }"
            >
              <!-- Text -->
              <Text
                id="input"
                ref="inputRef"
                is-input
                is-minimal
                placeholder="Message..."
                suppress-enter
                suppress-drop
                :model-value="draftText"
                @update:model-value="
                  (value) =>
                    state.update(
                      { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { draftText: value } },
                      { debounce: 'long' },
                    )
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
              <!-- Extras -->
              <div v-if="draftFiles.length > 0" class="mt-1 flex flex-row flex-wrap gap-x-2 gap-y-1">
                <!-- should also be a proper :FileGallery -->
                <div v-for="file in draftFiles" :key="file.id" class="group/file relative">
                  <File
                    :id="'file-' + file.id"
                    is-inline
                    :model-value="toNodeRef(file)"
                    :size="{
                      height:
                        size?.height != null && INLINE_FILE_TYPES.includes(file.type)
                          ? Math.min(200, size.height / 3)
                          : undefined,
                    }"
                  />
                  <!-- Remove -->
                  <button
                    class="absolute right-1 top-2 rounded-full bg-gray-100/80 px-1.5 py-0.5 text-gray-700 transition-colors duration-150 group-hover/file:bg-white/100 group-hover/file:text-gray-900"
                    @click.stop="removeFiles([file])"
                  >
                    <i class="fas fa-xmark" />
                  </button>
                </div>
              </div>
            </Scroll>
          </div>
        </div>
      </div>
      <!-- Spacing -->
      <div class="h-2" />
    </div>
  </div>
</template>
