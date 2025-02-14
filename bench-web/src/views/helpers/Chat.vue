<script lang="ts" setup>
import { supergraph } from "@/globals";
import { getBaseFromNode, isInlineSourceNode } from "@/language/core/const";
import { EditSubject, makeAndConditional, makeExpression } from "@/language/core/expression";
import { useSubnodeProperty } from "@/language/core/node";
import { emptyText, getTextLine, isTextEmpty, trimText } from "@/language/core/text";
import { INLINE_FILE_TYPES, uploadFile } from "@/language/resource/file";
import { createMessage, getMessageAuthorPtr } from "@/language/state/message";
import {
  AnyNodeData,
  DatabaseData,
  ExpressionData,
  ExpressionType,
  FileData,
  IconData,
  InlineSourceNodeData,
  MessageData,
  MessageProperty,
  MessageType,
  NodeReferenceData,
  NodeType,
  Orientation,
  PackageData,
  RectangleData,
  TextData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE, benchPtr } from "@/system/client";
import { SearchConnectionParams, useSearchConnection } from "@/system/connection";
import { bench, benchConnection, canvas, pkg, pkgGraph } from "@/system/space";
import { user } from "@/system/user";
import {
  ActionMapImplementation,
  fireAction,
  getAction,
  getNodesForAction,
  isActionEnabled,
  MESSAGE_CONTEXT_ACTIONS,
} from "@/ui/action";
import { useSingleDropZone } from "@/ui/drag";
import { isSelecting, startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { AvatarInline, getNodeIcon, getNodeName, IconInline } from "@/ui/icon";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import { formatAbsoluteDate, tsToDt } from "@/utils/time";
import RootHeader from "@/views/builtins/RootHeader.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import File from "@/views/content/File.vue";
import Text from "@/views/content/Text.vue";
import { useElementSize, useEventListener } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, nextTick, Ref, ref, toRef } from "vue";

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
  } & Partial<Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focus" | "selection" | "subnodePacked">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

// node
const channelPtr = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "channelPtr");
const threadPtr = useSubnodeProperty(NodeType.VIEW, ViewType.CHAT, subnodePacked, "threadPtr");
const channel = supergraph.getRef(channelPtr);
const thread = supergraph.getRef(threadPtr);
const nodePtr = computedValue(() => props.nodePtr);
const node = supergraph.getRef(nodePtr) as Ref<AnyNodeData>;
const basePtr = computed(() => (node.value != null ? getBaseFromNode(node.value) : null));
const base = pkgGraph.getRef(basePtr);
const nodeAncestors = pkgGraph.getAncestorsRef(nodePtr);
const scope: Ref<InlineSourceNodeData | PackageData | null> = computed(() => {
  if (isInlineSourceNode(node.value) || isNode(node.value, NodeType.PACKAGE)) {
    return node.value;
  } else if (isNode(node.value, NodeType.RECORD)) {
    return base.value as DatabaseData;
  } else {
    const scope = nodeAncestors.value.find((a) => isInlineSourceNode(a) || isNode(a, NodeType.PACKAGE));
    if (scope != null) {
      return scope;
    }
  }
  return pkg.value;
});
const scopePtr = computed(() => (scope.value != null ? toNodeRef(scope.value) : null));

//
// Messages
//

const DEFAULT_SORT = makeExpression({
  type: ExpressionType.ASCENDING,
  propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.createdAt),
});
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
  } else {
    filters.push(
      makeExpression({
        type: ExpressionType.NOT_EXISTS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.channelPtr),
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
  // scope
  if (channelPtr.value == null && threadPtr.value == null) {
    filters.push(
      makeExpression({
        type: ExpressionType.EQUALS,
        propertyPtr: propertyReference(NodeType.MESSAGE, MessageProperty.scopePtr),
        value: scopePtr.value,
      }),
    );
  }
  return makeAndConditional(filters);
});
const {
  roots: messages,
  connection,
  graph,
  isStale,
  isConnecting,
  isConnected,
} = useSearchConnection(
  { name: "chat", live: true },
  computed(
    (): SearchConnectionParams<NodeType.MESSAGE> => ({
      scope: BENCH_SCOPE.value,
      nodeType: NodeType.MESSAGE,
      count: true,
      isEnabled: scopePtr.value != null,
      sort: [DEFAULT_SORT],
      filter: filter.value,
    }),
  ),
);
const remoteAuthorsPtr: Ref<NodeReferenceData[]> = computed(() => {
  const authorsPtrById: Record<string, NodeReferenceData> = {};
  for (const message of messages.value) {
    if (message.createdByPtr != null && message.createdByPtr.nodeType == NodeType.USER) {
      authorsPtrById[message.createdByPtr.id!] = message.createdByPtr;
    }
  }
  return Object.values(authorsPtrById);
});
const remoteAuthors = supergraph.getManyRef(remoteAuthorsPtr);
const remoteAuthorsById: Ref<Record<string, EditSubject>> = computed(() =>
  remoteAuthors.value.reduce(
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
    const author = authorPtr != null ? (remoteAuthorsById.value[authorPtr.id!] ?? supergraph.get(authorPtr)) : null;
    const authorIcon = author != null ? (getNodeIcon(author) ?? null) : null;
    const authorName = author != null ? (getNodeName(author) ?? null) : null;
    let isNewGroup;
    let isNewDate;
    if (i == 0) {
      isNewGroup = true;
      isNewDate = false;
    } else {
      const previousDt = tsToDt(messages.value[i - 1].createdAt!);
      const currentDt = tsToDt(message.createdAt!);
      isNewGroup =
        message.createdByPtr?.id != messages.value[i - 1]?.createdByPtr?.id ||
        Math.abs(Number(messages.value[i - 1].createdAt!.seconds) - Number(message.createdAt!.seconds)) >
          MESSAGE_MAX_TIME_DELTA_SECONDS;
      isNewDate = previousDt.day != currentDt.day;
    }
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
// Interaction
//

const inputContainerRef = ref<HTMLInputElement | null>(null);
const inputRef = ref<InstanceType<typeof Text> | null>(null);
const inputSize = useElementSize(inputContainerRef);
const containerRef = ref<HTMLDivElement | null>(null);
const bodyScrollRef = ref<InstanceType<typeof Scroll> | null>(null);
const editingTextRefs = ref<InstanceType<typeof Text>[] | null>(null); // there can only be one but it's inside a v-for (so it has to be an array)
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);

const currentAuthor = user;
const editingText = ref<TextData | null>(null);
const editingPtr = ref<NodeReferenceData | null>(null);

// selecting
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });
const isSelectingChat = computed(() => isSelecting());

function focus() {
  inputRef.value?.focus?.();
}

function startEdit(message: MessageData) {
  editingPtr.value = toNodeRef(message);
  editingText.value = message.text ?? null;
  nextTick(() => {
    editingTextRefs.value?.[0]?.focus?.();
  });
}

function stopEdit() {
  editingPtr.value = null;
  editingText.value = null;
}

function submitEdit() {
  const message = messages.value.find((m) => m.id == editingPtr.value?.id);
  if (message == null) throw new Error("message not found");
  const text = trimText(editingText.value ?? emptyText());
  if (isTextEmpty(text)) return; // don't create empty messages
  connection.tx.update(message, { text });
}

function submit() {
  // create message
  if (benchPtr.value == null) throw new Error("no bench");
  if (scope.value == null) throw new Error("no scope");
  const text = trimText(draftText.value ?? emptyText());
  if (isTextEmpty(text)) return; // don't create empty messages
  createMessage(connection.tx, graph, {
    message: {
      type: MessageType.TEXT,
      parentPtr: benchPtr.value,
      channelPtr: channelPtr.value,
      threadPtr: threadPtr.value,
      scopePtr: toNodeRef(scope.value),
      replyToPtr: replyTo.value != null ? draftReplyTo.value : undefined,
      nodesPtr: draftNodesPtr.value,
      text,
    },
  });
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
    const upload = uploadFile(() => benchConnection.tx, file, { bench: bench.value });
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

// actions
const actions: Partial<ActionMapImplementation<"chat">> = {
  "chat.message.reply": {
    isEnabled: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      return messages.length > 0;
    },
    action: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      state.update(
        { metatype: NodeType.VIEW, type: ViewType.CHAT, subnode: { draftReplyToPtr: toNodeRef(messages[0]) } },
        { debounce: "tick" },
      );
      nextTick(() => {
        inputRef.value?.focus?.();
      });
    },
  },
  "chat.message.edit": {
    isEnabled: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      return messages.length > 0 && messages.every((m) => m.createdByPtr?.id == currentAuthor.value?.id);
    },
    action: (action, ctx) => {
      const { nodes: messages } = getNodesForAction(action, ctx, [NodeType.MESSAGE]);
      startEdit(messages[0]);
    },
  },
};

defineExpose<ViewExpose>({ self, id, actions, focus });
</script>
<template>
  <div ref="containerRef" class="relative">
    <!-- Root header -->
    <RootHeader
      v-if="isRoot"
      :self="self"
      :node-ptr="threadPtr ?? channelPtr"
      :focus="$props.focus"
      :graph="pkgGraph"
    />

    <!-- Drop zone -->
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

    <!-- Body -->
    <Scroll
      id="scroll"
      ref="bodyScrollRef"
      class="relative"
      :orientation="Orientation.VERTICAL"
      stick-to-end
      :size="{
        height:
          size?.height != null
            ? size.height - inputSize.height.value - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0)
            : undefined,
      }"
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZone, e)"
    >
      <!-- Messages -->
      <ul class="relative mb-3 mt-2 flex flex-col">
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
          data-ignore-element="self"
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
              {{ getTextLine(replyTo.message.text) }}
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
              <AvatarInline v-if="isNewGroup" class="mr-1 text-gray-700" size="medium" v-bind="authorIcon" />
              <!-- Time/edited otherwise -->
              <div v-else class="pt-0.5 text-xs text-gray-400">
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
                <span class="text-base font-medium">
                  {{ authorName ?? "???" }}
                </span>
                <!-- Timestamp -->
                <span class="ml-1.5 text-xs text-gray-400">
                  {{ formatAbsoluteDate(message.createdAt!, { prefer: "time" }) }}
                </span>
                <!-- Edited? -->
                <span v-if="isEdited" class="fas fa-pencil ml-1 text-xs text-gray-300" />
              </div>
              <!-- Actions -->
              <div
                v-if="!isEditing"
                class="absolute right-0 top-0 z-10 flex flex-row rounded border border-gray-200 bg-white opacity-0 group-hover/message:opacity-100"
              >
                <button
                  v-for="action in MESSAGE_CONTEXT_ACTIONS.map(getAction)"
                  :key="action.id"
                  v-tooltip="{ small: true, title: action.title, group: 'message' }"
                  class="rounded px-1.5 py-0.5 transition-colors duration-150 enabled:cursor-pointer enabled:text-gray-700 enabled:hover:bg-gray-100 disabled:cursor-not-allowed disabled:text-gray-400"
                  :disabled="!isActionEnabled(action, { nodes: [message] })"
                  @click.stop.prevent="fireAction(action, { nodes: [message] })"
                >
                  <IconInline v-bind="action.icon" />
                </button>
              </div>
              <!-- Content -->
              <template v-if="!isEditing">
                <Text v-if="!isEditing" :id="'text-' + message.id" is-minimal :model-value="message.text" />
              </template>
              <div v-else class="my-1">
                <Text
                  :id="'text-' + message.id"
                  ref="editingTextRefs"
                  :model-value="editingText ?? undefined"
                  is-input
                  suppress-enter
                  suppress-drop
                  @update:model-value="
                    (value) => {
                      editingText = value;
                    }
                  "
                  @keydown.enter="
                    (e: KeyboardEvent) => {
                      if (!e.shiftKey) {
                        submitEdit();
                        stopEdit();
                      }
                    }
                  "
                  @keydown.esc.stop.prevent="stopEdit()"
                />
                <div v-if="isEditing" class="mt-0.5 flex-row text-xs text-gray-400">
                  <span>
                    escape to
                    <a href="#" class="text-primary-700 underline-offset-2 hover:underline" @click.stop="stopEdit()"
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
              <div v-if="filesPtr.length > 0" class="mb-2 mt-0.5 flex flex-row flex-wrap gap-x-2 gap-y-2">
                <File
                  v-for="filePtr in filesPtr"
                  :id="'file-' + filePtr.id"
                  :key="filePtr.id"
                  is-minimal
                  is-inline
                  :model-value="toNodeRef(filePtr)"
                />
              </div>
            </div>
          </div>
        </li>
      </ul>

      <!-- Empty -->
      <div
        v-if="messages.length == 0"
        class="mx-1.5 flex w-full flex-row items-center justify-center px-2.5"
        :style="{
          height: MESSAGE_HEIGHT_MIN + 'px',
        }"
      >
        <!-- Loading -->
        <span v-if="!isConnected || isStale" class="">
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </span>
        <!-- Empty -->
        <span v-else class="text-gray-400">No Messages here yet</span>
      </div>

      <!-- Selection -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
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
          {{ getTextLine(replyTo.message.text) }}
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
                <div v-for="file in draftFiles" :key="file.id" class="group/file relative">
                  <File
                    :id="'file-' + file.id"
                    is-minimal
                    is-inline
                    :model-value="toNodeRef(file)"
                    :size="{
                      height:
                        size?.height != null && INLINE_FILE_TYPES.includes(file.type)
                          ? Math.min(300, size.height / 2)
                          : undefined,
                    }"
                  />
                  <!-- Remove -->
                  <button
                    class="absolute right-1 top-1 rounded-full bg-white/80 px-1.5 py-0.5 text-gray-700 transition-colors duration-150 group-hover/file:bg-white/100 group-hover/file:text-gray-900"
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
      <div class="h-4" />
    </div>
  </div>
</template>
