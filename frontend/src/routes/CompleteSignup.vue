<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import NotificationArea from "@/components/notifications/NotificationArea.vue";
import { useValidName, useValidSlug } from "@/composables/useValidation";
import { useAuth, useRedirectIfNotLoggedIn } from "@/state/auth";
import { useOperations } from "@/state/operations";
import { useTitle } from "@vueuse/core";
import { computed, onMounted, ref, watchEffect, type Ref } from "vue";
import { useRouter } from "vue-router";

defineProps<{ next?: string }>();
const title = useTitle();
title.value = "Bench - Sign up";

const auth = useAuth();
const router = useRouter();

useRedirectIfNotLoggedIn({ name: "Home" });

const name: Ref<string | null> = ref(auth.me.value?.name || null);
const username: Ref<string | null> = ref(auth.me.value?.username || null);
// set name/username once when is loaded (can only get here if logged in)
watchEffect(() => {
  if (auth.me.value != null && name.value == null) {
    username.value = auth.me.value.username;
    name.value = auth.me.value.name;
  }
});

// auto-fill on tab
function autofillName(event: KeyboardEvent) {
  if (
    name.value != auth.me.value?.name &&
    (name.value?.length == 0 || auth.me.value?.name.startsWith(name.value ?? ""))
  ) {
    name.value = auth.me.value?.name as string;
    event.stopPropagation();
    event.preventDefault();
  }
}
function autofillUsername(event: KeyboardEvent) {
  if (
    username.value != auth.me.value?.username &&
    (username.value?.length == 0 || auth.me.value?.username.startsWith(username.value ?? ""))
  ) {
    username.value = auth.me.value?.username as string;
    event.stopPropagation();
    event.preventDefault();
  }
}

const nameValidation = useValidName(name);
const slugValidation = useValidSlug(
  username,
  computed(() => auth.me.value ?? null)
);

// auto-focus name on load
const nameRef: Ref<HTMLInputElement | null> = ref(null);
onMounted(() => {
  nameRef.value?.focus();
  // select all
  nameRef.value?.setSelectionRange(0, nameRef.value?.value?.length ?? 0);
});

const completing = ref(false);
const canComplete = computed(
  () =>
    !completing.value &&
    nameValidation.valid.value &&
    slugValidation.valid.value &&
    !slugValidation.loading.value &&
    slugValidation.available.value
);

const ops = useOperations();
async function completeSignup() {
  if (auth.me.value == null || username.value == null || name.value == null) {
    throw new Error("cannot complete with missing data");
  }
  completing.value = true;
  await ops.user.completeSignup(auth.me.value?.id as string, username.value as string, name.value as string);
  // redirect to home
  router.push({ name: "Home" });
}
</script>
<template>
  <div class="flex h-full flex-col bg-white pb-12">
    <FatHeader>
      <template v-slot:left>
        <HomeButton />
      </template>
      <template v-slot:right> </template>
    </FatHeader>
    <div class="mx-auto mt-20 w-72 text-center lg:mt-32">
      <div class="flex flex-row items-baseline justify-center gap-1">
        <div class="font-mono text-2xl font-bold">
          <span class="-mx-0.5 text-gray-900">[</span>
          <span class="text-3xl text-orange-600">x</span>
          <span class="-mx-0.5 text-gray-900">]</span>
        </div>
        <h3 class="font-mono text-2xl font-bold">Bench</h3>
      </div>
      <h1 class="mt-4 text-5xl font-bold">Welcome</h1>
      <p class="mt-4 text-lg text-orange-700">AI is yours for the making.</p>

      <!-- Fields to complete -->
      <div class="mt-10 flex flex-col gap-4">
        <!-- Full name -->
        <div class="text-left">
          <span class="text-md text-gray-700">Your name</span>
          <input
            ref="nameRef"
            type="text"
            minlength="3"
            maxlength="128"
            :placeholder="auth.me.value?.name || 'Yatima'"
            v-model="name"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            @keydown.tab.exact="autofillName"
            spellcheck="false"
          />
          <FadeTransition mode="out-in">
            <span class="mt-1 text-sm text-yellow-600" v-if="!nameValidation.valid.value"
              >The bots don't like this name.</span
            >
            <span class="mt-1 text-sm text-gray-500" v-else>Great name.</span>
          </FadeTransition>
        </div>

        <!-- Username -->
        <div class="text-left">
          <span class="text-md text-gray-700">Pick a username</span>
          <input
            type="text"
            minlength="3"
            maxlength="128"
            pattern="[a-z0-9_-]+"
            :placeholder="auth.me.value?.username || 'yatima'"
            v-model="username"
            class="mt-1 w-full rounded-sm border border-orange-600 py-1 placeholder:text-gray-400 focus:border-orange-600 focus:bg-orange-100 focus:outline-none focus:ring-0"
            @keydown.tab.exact="autofillUsername"
            spellcheck="false"
          />
          <FadeTransition mode="out-in">
            <span v-if="!slugValidation.valid.value" class="mt-1 text-sm text-orange-600">
              Invalid username. <span class="font-mono text-xs text-gray-500">[a-z0-9_-]{3,}</span>
            </span>
            <span v-else-if="slugValidation.loading.value" class="mt-1">&nbsp;</span>
            <span v-else-if="!slugValidation.available.value" class="mt-1 text-sm text-red-600">
              That username is
              <router-link
                :to="`/${username}`"
                class="underline decoration-dotted underline-offset-2 hover:decoration-solid focus:decoration-solid focus:outline-none"
                >taken</router-link
              >.
            </span>
            <span v-else class="mt-1 text-sm text-gray-500">Yours for the taking.</span>
          </FadeTransition>
        </div>

        <button
          :disabled="!canComplete"
          class="mt-6 w-fit self-end border border-orange-600 px-3 py-1 hover:bg-orange-600 hover:text-white focus:bg-orange-600 focus:text-white focus:outline-none"
          :class="{ 'pointer-events-none opacity-50': !canComplete }"
          @click="completeSignup"
        >
          Enter &rarr;
        </button>
      </div>
    </div>
    <NotificationArea />
  </div>
</template>
