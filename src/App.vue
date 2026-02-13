<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { configManager } from "./utils/ConfigManager";
import TitleBar from "./components/TitleBar.vue";
import FilterPage from "./views/FilterPage.vue";
import MarketPage from "./views/MarketPage.vue";
import SettingsOverlay from "./views/SettingsOverlay.vue";

// State
const currentView = ref("filter");
const showSettings = ref(false);

const toggleSettings = () => {
  showSettings.value = !showSettings.value;
};

// Component Mapping
const viewComponents: Record<string, any> = {
  filter: FilterPage,
  market: MarketPage
};

// Window Management
onMounted(async () => {
  // 1. Initialize Config
  await configManager.init();
  const settings = configManager.getSettings();
  const appWindow = getCurrentWindow();

  // 2. Restore Size (and maximize state if we had it)
  if (settings.width && settings.height) {
     // Apply size
     await appWindow.setSize(new LogicalSize(settings.width, settings.height));
  }
  
  if (settings.maximized) {
      await appWindow.maximize();
  }

  // 3. Setup Close Listener to save settings
  // Note: We don't prevent close, just save on the fly.
  // If preventing is needed to ensure write completion, we'd need deeper integration.
  // But for now, firing the async write is usually enough before the process kills.
  appWindow.listen('tauri://close-requested', async () => {
      try {
          const factor = await appWindow.scaleFactor();
          const size = await appWindow.innerSize();
          const logical = size.toLogical(factor);
          const isMaximized = await appWindow.isMaximized();
          
          await configManager.saveSettings({
              width: logical.width,
              height: logical.height,
              maximized: isMaximized
          });
      } catch (e) {
          console.error("Failed to save config on close", e);
      }
      // We don't block close here. 
      // If we needed to block: event.preventDefault(), save, then window.destroy().
  });
});
</script>

<template>
  <!-- Element Plus Dark Mode Provider Wrapper -->
  <el-config-provider namespace="ep">
    <div class="app-container dark">
      <!-- Layer 1: Background Image -->
      <div class="bg-layer"></div>

      <!-- Layer 2: Vignette Shadow -->
      <div class="shadow-layer"></div>

      <!-- Layer 3: App Content -->
      <div class="app-layout">
        <TitleBar 
          v-model:currentView="currentView"
          :isSettingsOpen="showSettings"
          @toggleSettings="toggleSettings" 
        />

        <div class="content-area">
          <component :is="viewComponents[currentView]" />
          
          <Transition name="slide">
            <SettingsOverlay v-if="showSettings" />
          </Transition>
        </div>
      </div>
    </div>
  </el-config-provider>
</template>

<style>
/* Global Resets */
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  color: #cfd3dc;
  background-color: transparent;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  height: 100vh;
  width: 100vw;
  background: #000;
}

#app {
  height: 100%;
  width: 100%;
}

/* Scrollbar styling for WebKit */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent; 
}
::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2); 
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3); 
}
</style>

<style scoped>
.app-container {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  /* Force dark scheme context */
  color-scheme: dark;
}

/* Layer 1 */
.bg-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: url('/Background.png');
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  z-index: 0;
}

/* Layer 2 */
.shadow-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  /* Radial gradient: Transparent center, Dark edges */
  background: radial-gradient(circle at center, rgba(10,10,10,0.2) 20%, rgba(0,0,0,0.85) 100%);
  pointer-events: none;
  z-index: 1;
}

/* Layer 3 */
.app-layout {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  z-index: 2;
}

.content-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}

/* Transition for Settings */
.slide-enter-active,
.slide-leave-active {
  transition: transform 0.3s ease;
}

.slide-enter-from,
.slide-leave-to {
  transform: translateX(100%);
}
</style>
