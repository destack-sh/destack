<script lang="tsx" setup>
import { runAction } from "@/system/action";
import type { GraphConnection } from "@/system/connection";
import { IconInline, makeIcon } from "@/system/icon";
import { DEFAULT_USER_ICON } from "@/system/lang";
import { clientInfo, clientMeta } from "@/system/local";
import { bench } from "@/system/space";
import { client, user } from "@/system/user";
import { menuItemFromAction } from "@/utils/menu";
import Button from "@/views/controls/Button.vue";
import Dock from "@/views/private/Dock.vue";
import Menu from "@/views/private/Menu.vue";
import Popover from "@/views/private/Popover.vue";

const props = defineProps<{
  spaceConnection: GraphConnection;
}>();
</script>
<template>
  <div
    class="flex w-full flex-row items-center justify-between gap-x-4 bg-gray-100 px-4 text-sm text-gray-900"
    data-outside-view="true"
  >
    <!-- Left -->
    <div class="flex flex-shrink-0">
      <!-- Bench -->
      <button v-if="bench" class="rounded-md border border-gray-300 bg-white px-2 py-1">{{ bench.slug }}</button>
      <div
        v-else
        class="select-none rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300"
        v-tooltip="{ icon: 'fas fa-triangle-person-digging', title: 'It`s a Beta', text: 'Welcome to the future.' }"
      >
        <span class="font-semibold">Bench</span>
        <span class="ml-1 pl-0.5 font-semibold underline decoration-primary-400 decoration-2">Beta</span>
      </div>
      <!-- Status -->
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div class="flex flex-1 flex-shrink-0 items-center justify-center -sm:hidden">
      <!-- Dock -->
      <Dock class="w-fit px-2 py-1" />
    </div>

    <!-- Right -->
    <div class="flex flex-shrink-0 flex-row">
      <template v-if="user">
        <!-- User (logged in) -->
        <Popover placement="bottom-right" :reference-margin="4">
          <template v-slot:trigger="{ toggle }">
            <button
              class="flex flex-row items-center rounded-md border border-gray-300 bg-white px-2 py-1 text-gray-900 shadow-sm shadow-gray-300 hover:cursor-pointer hover:border-gray-400 hover:bg-gray-100 focus:shadow-primary-600"
              @click="toggle"
            >
              <span class="mr-1.5 rounded-md border border-gray-300 bg-primary-300 px-0.5">
                <IconInline class="" v-bind="user.icon ?? DEFAULT_USER_ICON" />
              </span>
              <span>{{ user.name ?? user.slug }}</span>
            </button>
          </template>
          <template v-slot:content="{ close }">
            <Menu
              @close="close"
              :items="[
                menuItemFromAction('user.goToHome', { category: 'primary' }),
                menuItemFromAction('user.activate', { category: 'primary' }),
                menuItemFromAction('space.launch.notifications', { category: 'primary' }),
                menuItemFromAction('user.editKeybindings', { category: 'secondary' }),
                menuItemFromAction('user.logout', { category: 'secondary' }),
              ]"
            >
              <!-- User Info -->
              <template #header>
                <div class="px-2.5 pb-2 pt-1.5">
                  <div class="flex flex-row">
                    <div class="mr-2 h-fit rounded-md border border-gray-700 bg-primary-300 px-2.5 py-0.5 text-xl">
                      <IconInline class="" v-bind="user.icon ?? DEFAULT_USER_ICON" />
                    </div>
                    <div class="flex flex-col leading-tight">
                      <span class="font-medium">{{ user.name }}</span>
                      <span class="text-gray-500">{{ user.slug }}</span>
                    </div>
                  </div>
                </div>
              </template>
              <!-- Client Info -->
              <template #footer>
                <div class="px-2.5 pb-1.5 pt-2 text-gray-500">
                  <div class="flex w-full flex-row">
                    <span>{{ clientMeta.operatingSystem }}</span>
                    <span class="ml-auto">{{ clientMeta.browserName }} {{ clientMeta.browserVersion }}</span>
                  </div>
                  <div class="flex w-full flex-row text-xs">
                    <span>id:{{ client?.id.split("-")[0] }}</span>
                    <span class="ml-auto">nonce:{{ clientMeta.nonce.split("-")[0] }}</span>
                  </div>
                </div>
              </template>
            </Menu>
          </template>
        </Popover>
      </template>
      <!-- User (unauthenticated) -->
      <template v-else>
        <Button
          title="Log In"
          :icon="makeIcon({ name: 'fas fa-arrow-right-from-bracket' })"
          @click="() => runAction('user.login')"
        />
      </template>
    </div>
  </div>
</template>
