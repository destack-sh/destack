<script lang="ts" setup>
import type { SimpleType } from "@/components/statement";
import { useAppearance } from "@/state/appearance";
import { useSecrets, type SecretRecord } from "@/state/secret";
import { syncProperty } from "@/utils/sync";
import { PlusIcon, KeyIcon, EyeIcon, EyeSlashIcon, DocumentDuplicateIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref } from "vue";

const props = defineProps<{
  modelValue: SecretRecord | null;
  type: SimpleType;
  readonly: boolean;
  preview: boolean;
  active: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
  (e: "enter"): void;
}>();

const ops = useSecrets();

const setButtonRef = ref<HTMLButtonElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const secretValue = ref<string | null>(null);
const hidden = ref(true);

// auto write secret when editing
syncProperty({
  value: secretValue,
  editing: ref(props.preview),
  read: () => doReveal(true),
  write: writeSecretValue,
  debounceMs: 1000,
});

async function writeSecretValue() {
  const newRecord = await ops.upsert(props.modelValue, {
    name: props.type.name ?? null,
    value: secretValue.value,
  });
  emit("update:modelValue", {
    id: newRecord.id,
    name: newRecord.name,
    sha512: newRecord.sha512,
  } as SecretRecord);
}

function focus() {
  doReveal(); // pre-reveal
  nextTick(() => inputRef.value?.focus());
}

function blur() {
  setButtonRef.value?.blur();
}

async function doReveal(reload?: boolean) {
  if ((reload || secretValue.value == null) && props.modelValue != null) {
    secretValue.value = await ops.reveal(props.modelValue.id);
  }
}

async function reveal() {
  await doReveal();
  hidden.value = false;
}

function hide() {
  hidden.value = true;
}

async function copy() {
  await doReveal();
  if (secretValue.value != null) {
    navigator.clipboard.writeText(secretValue.value);
  }
}

const inlineActions = computed(() => [
  {
    label: hidden.value ? "Reveal" : "Hide",
    icon: hidden.value ? EyeIcon : EyeSlashIcon,
    action: hidden.value ? reveal : hide,
  },
  {
    label: "Copy",
    icon: DocumentDuplicateIcon,
    action: copy,
  },
]);

const appearance = useAppearance();

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div
    class="group/iface group flex w-full flex-row gap-x-2.5"
    :class="{
      'items-center justify-center': modelValue == null,
      'justify-end bg-gray-100': modelValue != null && preview,
      'justify-between': modelValue != null && !preview,
      'min-w-[300px]': !preview,
    }"
  >
    <button
      v-if="!readonly && modelValue == null && preview"
      ref="setButtonRef"
      class="flex flex-row gap-0.5 p-0.5 text-gray-400 opacity-0 hover:bg-orange-100 focus:bg-orange-100 group-hover:opacity-100"
    >
      <PlusIcon class="h-4 w-4" />
      <KeyIcon class="h-4 w-4" />
    </button>
    <!-- Editable input -->
    <input
      v-if="!preview || modelValue != null"
      ref="inputRef"
      v-model="secretValue"
      class="w-full rounded-none border-none bg-transparent p-0 text-gray-900 outline-none ring-0 placeholder:text-gray-300 focus:ring-0"
      :type="hidden ? 'password' : 'text'"
      :placeholder="hidden ? '••••••••••••••••••••••••••••' : '123456789-123456789'"
      :class="{
        'font-mono': appearance.fontMono,
        'text-sm': appearance.textSmall,
        'text-md': !appearance.textSmall,
      }"
      @keydown.enter.stop.prevent="writeSecretValue(), emit('enter')"
    />
    <!-- Controls -->
    <div class="flex flex-row gap-0.5" v-if="modelValue != null || !preview">
      <button
        v-for="action in inlineActions"
        :key="action.label"
        class="p-0.5 text-gray-400 hover:bg-orange-100 focus:bg-orange-100"
        @click.stop="action.action"
        @keydown.enter.stop="action.action"
      >
        <component :is="action.icon" class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>
