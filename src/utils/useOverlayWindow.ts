import { onMounted, onUnmounted, Ref, ref, watch } from "vue";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

interface OverlayOptions {
  label: string;
  url: string;
  containerRef: Ref<HTMLElement | null>;
  isActive: Ref<boolean>;
}

export function useOverlayWindow(options: OverlayOptions) {
  const overlayWindow = ref<WebviewWindow | null>(null);

  const state: {
    moveListener: (() => void) | null;
    resizeListener: (() => void) | null;
    focusListener: (() => void) | null;
    blurListener: (() => void) | null;
    overlayFocusUnlisten: (() => void) | null;
    overlayBlurUnlisten: (() => void) | null;
    blurTimeout: ReturnType<typeof setTimeout> | null;
  } = {
    moveListener: null,
    resizeListener: null,
    focusListener: null,
    blurListener: null,
    overlayFocusUnlisten: null,
    overlayBlurUnlisten: null,
    blurTimeout: null,
  };

  const mainWindow = getCurrentWindow();

  const updateWindowBounds = async () => {
    if (!overlayWindow.value || !options.containerRef.value || !options.isActive.value) return;

    try {
        // Z-Order/Minimization handling
        if (await mainWindow.isMinimized()) {
            await overlayWindow.value.hide();
           return;
        }

        const rect = options.containerRef.value.getBoundingClientRect();
        const scaleFactor = await mainWindow.scaleFactor();
        const winPos = await mainWindow.outerPosition();
        
        const winX = winPos.x / scaleFactor;
        const winY = winPos.y / scaleFactor;
        
        const targetX = winX + rect.x;
        const targetY = winY + rect.y;
        
        await overlayWindow.value.setPosition(new LogicalPosition(targetX, targetY));
        await overlayWindow.value.setSize(new LogicalSize(rect.width, rect.height));
    } catch (e) {
        console.error("Bounds update failed", e);
    }
  };

  const checkAppFocus = async () => {
    if (!overlayWindow.value) return;
    
    let isActive = false;
    
    try {
        // If minimized, hide and drop z-order
        if (await mainWindow.isMinimized()) {
          await overlayWindow.value.hide();
          await overlayWindow.value.setAlwaysOnTop(false);
            return;
        }
        const mainState = await mainWindow.isFocused();
        const overlayState = await overlayWindow.value.isFocused();
        isActive = mainState || overlayState;
    } catch (e) {
        console.warn("Could not query focus state, falling back to events", e);
        isActive = false; 
    }

    if (isActive && options.isActive.value) {
        await overlayWindow.value.setAlwaysOnTop(true);
        await overlayWindow.value.show();
        updateWindowBounds();
    } else {
        await overlayWindow.value.setAlwaysOnTop(false);
    }
  };

  const handleFocus = () => {
    if (state.blurTimeout) {
        clearTimeout(state.blurTimeout);
        state.blurTimeout = null;
    }
    // Immediate check when gaining focus
    checkAppFocus();
  };

  const handleBlur = () => {
    // Debounce blur to allow focus to swap between windows
    if (state.blurTimeout) clearTimeout(state.blurTimeout);
    state.blurTimeout = setTimeout(() => {
        checkAppFocus();
    }, 100);
  };

  const setupDomListeners = () => {
    window.addEventListener('focus', handleFocus);
    window.addEventListener('blur', handleBlur);
  };

  const cleanupDomListeners = () => {
    window.removeEventListener('focus', handleFocus);
    window.removeEventListener('blur', handleBlur);
  };

  const setupOverlayListeners = async () => {
    if (!overlayWindow.value) return;
    
    if (state.overlayFocusUnlisten) state.overlayFocusUnlisten();
    if (state.overlayBlurUnlisten) state.overlayBlurUnlisten();
    
    // Listen to Tauri events on the child window
    state.overlayFocusUnlisten = await overlayWindow.value.listen('tauri://focus', handleFocus);
    state.overlayBlurUnlisten = await overlayWindow.value.listen('tauri://blur', handleBlur);
  };

  const initWindow = async () => {
    const { label, url } = options;
    
    // Try existing window first
    let existing = await WebviewWindow.getByLabel(label);
    
    // Create via backend (injects ad stub) so both pages share creation flow
    if (!existing) {
        try {
            await invoke('create_overlay_window', { label, targetUrl: url });
            existing = await WebviewWindow.getByLabel(label);
        } catch (e) {
            console.error("Window creation error", e);
        }
    }

    if (existing) {
           overlayWindow.value = existing;
        await setupOverlayListeners();
        
        if (options.isActive.value) {
             await handleFocus();
             await overlayWindow.value.show();
             await overlayWindow.value.setFocus();
        }
    }
  };

  // Watch visibility
  watch(options.isActive, async (active) => {
        if (!overlayWindow.value) {
          if (active) await initWindow();
          else return;
      }
      
      if (active) {
          await updateWindowBounds();
          await overlayWindow.value?.show();
          await overlayWindow.value?.setFocus();
      } else {
          await overlayWindow.value?.hide();
      }
  });

  // Setup listeners
  onMounted(async () => {
      state.moveListener = await mainWindow.listen('tauri://move', updateWindowBounds);
      state.resizeListener = await mainWindow.listen('tauri://resize', updateWindowBounds);
      
      // Z-Order syncing - Main Window Events
      state.focusListener = await mainWindow.listen('tauri://focus', handleFocus);
      state.blurListener = await mainWindow.listen('tauri://blur', handleBlur);
      setupDomListeners();
      
      // Initial check
      if (options.isActive.value) {
          await initWindow();
      }
  });

  onUnmounted(async () => {
      if (state.moveListener) state.moveListener();
      if (state.resizeListener) state.resizeListener();
      if (state.focusListener) state.focusListener();
      if (state.blurListener) state.blurListener();
      if (state.overlayFocusUnlisten) state.overlayFocusUnlisten();
      if (state.overlayBlurUnlisten) state.overlayBlurUnlisten();
      cleanupDomListeners();
      
        if (overlayWindow.value) {
          await overlayWindow.value.hide();
      }
  });

  return {
    overlayWindow,
    updateWindowBounds,
    initWindow,
  };
}
