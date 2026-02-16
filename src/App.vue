<script setup lang="ts">
import { ref, onMounted, computed, watchEffect } from "vue";
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { convertFileSrc } from '@tauri-apps/api/core';
import { configManager } from "./utils/ConfigManager";
import TitleBar from "./components/TitleBar.vue";
import FilterPage from "./views/FilterPage.vue";
import MarketPage from "./views/MarketPage.vue";
import WorkshopPage from "./views/WorkshopPage.vue";
import PoedbPage from "./views/PoedbPage.vue";
import SettingsPage from "./views/SettingsPage.vue";

// State
const currentView = ref("filter");
const settings = configManager.getSettings();

const bgStyle = computed(() => {
    const type = settings.backgroundType;
    const path = settings.backgroundPath;

    if (type === 'image' && path) {
        return { 
          backgroundImage: `url("${convertFileSrc(path)}")`,
          backgroundSize: 'cover',
          backgroundPosition: 'center'
        };
    }
    // If video is active, show nothing behind (or show default while loading? but we want clean)
    if (type === 'video' && path) {
        return {}; 
    }
    // Default or Fallback (including when Image/Video selected but no path)
    return { 
      backgroundImage: `url("/Background.png")`,
      backgroundSize: 'cover',
      backgroundPosition: 'center'
    };
});

const videoRef = ref<HTMLVideoElement | null>(null);

// Apply volume when changed or when video mounts
watchEffect(() => {
    if (videoRef.value) {
         const vol = (settings.backgroundVolume ?? 0) / 100;
         videoRef.value.volume = vol;
         videoRef.value.muted = (vol === 0);
    }
});

const isVideo = computed(() => settings.backgroundType === 'video' && !!settings.backgroundPath);
const videoSrc = computed(() => settings.backgroundPath ? convertFileSrc(settings.backgroundPath) : '');

// Component Mapping
const viewComponents: Record<string, any> = {
  filter: FilterPage,
  settings: SettingsPage,
};

// Window Management
onMounted(async () => {
  // 1. Initialize Config
  await configManager.init();
  const settings = configManager.getSettings();
  const appWindow = getCurrentWindow();

  // 2. Restore Size (and maximize state if we had it)
  if (settings.width && settings.height) {
     // Safety check: ensure we don't restore to a tiny/unusable window
     // Using min dimensions from tauri.conf.json (1056x594) as baseline
     const kMinWidth = 1056;
     const kMinHeight = 594;
     
     const safeWidth = Math.max(settings.width, kMinWidth);
     const safeHeight = Math.max(settings.height, kMinHeight);

     // Apply size
     await appWindow.setSize(new LogicalSize(safeWidth, safeHeight));
  }
  
  if (settings.maximized) {
      await appWindow.maximize();
  }

  // 3. Setup Close Listener to save settings
  // Use onCloseRequested to handle the close event properly with Tauri v2
  appWindow.onCloseRequested(async (event) => {
      // Prevent the window from closing immediately so we can save settings
      event.preventDefault();
      
      try {
          const factor = await appWindow.scaleFactor();
          const size = await appWindow.innerSize();
          const logical = size.toLogical(factor);
          const isMaximized = await appWindow.isMaximized();
          const isMinimized = await appWindow.isMinimized();
          
          // Only save dimensions if window is in a valid state (not minimized, and has reasonable size)
          if (!isMinimized && logical.width > 100 && logical.height > 100) {
              await configManager.saveSettings({
                  width: logical.width,
                  height: logical.height,
                  maximized: isMaximized
              });
          }
      } catch (e) {
          console.error("Failed to save config on close", e);
      } finally {
          // Close child windows first
          const marketOverlay = await WebviewWindow.getByLabel('market-overlay');
          if (marketOverlay) {
              await marketOverlay.destroy();
          }

          const poedbOverlay = await WebviewWindow.getByLabel('poedb-overlay');
          if (poedbOverlay) {
              await poedbOverlay.destroy();
          }
          
          // Force close the window strictly after saving
          appWindow.destroy();
      }
  });
});
</script>

<template>
  <div class="app-container dark">
    <!-- Layer 1: Background Image -->
    <div class="bg-layer" :style="!isVideo ? bgStyle : {}">
      <video v-if="isVideo" ref="videoRef" :src="videoSrc" autoplay loop muted playsinline class="bg-video"></video>
    </div>

    <!-- Layer 2: Vignette Shadow -->
    <div class="shadow-layer"></div>

    <!-- Layer 3: App Content -->
    <div class="app-layout">
      <TitleBar 
        v-model:currentView="currentView"
      />

      <div class="content-area">
        <MarketPage v-show="currentView === 'market'" :is-active="currentView === 'market'" class="persistent-view" />
        <WorkshopPage v-show="currentView === 'workshop'" class="persistent-view" />
        <PoedbPage v-show="currentView === 'poedb'" :is-active="currentView === 'poedb'" class="persistent-view" />
        <transition name="page-fade" mode="out-in">
          <keep-alive>
              <component :is="viewComponents[currentView]" :key="currentView" />
          </keep-alive>
        </transition>
      </div>
    </div>
  </div>
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
  /* Deep void base with subtle cold light emitting from center */
  background-color: #050505;
  background-size: cover;
  background-position: center;
  background: radial-gradient(
      ellipse at 50% 40%, 
      #232838 0%,       /* Muted blue-grey core - subtle light */
      #111216 40%,      /* Transition to dark slate */
      #020202 100%      /* Pitch black edges */
  );
  z-index: 0;
}

.bg-video {
  width: 100%;
  height: 100%;
  object-fit: cover;
  position: absolute;
  top: 0;
  left: 0;
}

/* Layer 2 */
.shadow-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  /* Smooth vignette to focus attention inward */
  background: radial-gradient(
      circle at center, 
      transparent 10%, 
      rgba(0,0,0,0.4) 50%, 
      rgba(0,0,0,0.95) 100%
  );
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

/* Page fade + subtle vertical movement for view changes (short, snappy) */
.page-fade-enter-active,
.page-fade-leave-active {
  /* Only animate transform to avoid opacity-related repaint that
     conflicts with backdrop-filter (glass) rendering. Keep same
     timing to preserve a snappy feel. */
  transition: transform 180ms ease;
}
.page-fade-enter-from {
  transform: translateY(6px);
}
.page-fade-enter-to {
  transform: translateY(0);
}
.page-fade-leave-from {
  transform: translateY(0);
}
.page-fade-leave-to {
  transform: translateY(-6px);
}

.persistent-view {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 10;
}
</style>
