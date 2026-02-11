import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  DuplicateGroup,
  ScanProgress,
  ScanComplete,
  DetectionResult,
  ScanPhaseEvent,
  DetectionProgressEvent,
  ScanErrorEvent,
} from '$lib/types';

// Store state
interface ScanState {
  isScanning: boolean;
  phase: ScanPhaseEvent['phase'] | 'idle' | 'error' | 'detecting';
  phaseMessage: string;
  progress: ScanProgress | null;
  detectionProgress: DetectionProgressEvent | null;
  liveGroups: DuplicateGroup[];
  finalResult: DetectionResult | null;
  scanComplete: ScanComplete | null;
  error: string | null;
}

function createScanStore() {
  const { subscribe, set, update } = writable<ScanState>({
    isScanning: false,
    phase: 'idle',
    phaseMessage: '',
    progress: null,
    detectionProgress: null,
    liveGroups: [],
    finalResult: null,
    scanComplete: null,
    error: null,
  });

  let unlisteners: UnlistenFn[] = [];
  let isInitialized = false;

  return {
    subscribe,

    // Initialize event listeners (call once on app mount)
    async init() {
      // Prevent double initialization
      if (isInitialized) {
        console.warn('scanStore already initialized');
        return;
      }

      // Clean up any existing listeners
      this.cleanup();

      try {
        unlisteners = [
          // File discovery progress
          await listen<ScanProgress>('scan-progress', (e) => {
            update((state) => ({ ...state, progress: e.payload }));
          }),

          // Phase transitions
          await listen<ScanPhaseEvent>('scan-phase', (e) => {
            update((state) => ({
              ...state,
              phase: e.payload.phase,
              phaseMessage: e.payload.message,
            }));
          }),

          // Live duplicate streaming
          await listen<DuplicateGroup>('duplicate-found', (e) => {
            update((state) => ({
              ...state,
              liveGroups: [...state.liveGroups, e.payload].sort(
                (a, b) => b.wasted_space - a.wasted_space,
              ),
            }));
          }),

          // Detection progress (hashing stats)
          await listen<DetectionProgressEvent>('detection-progress', (e) => {
            update((state) => ({ ...state, detectionProgress: e.payload }));
          }),

          // Final results
          await listen<DetectionResult>('scan-results', (e) => {
            update((state) => ({
              ...state,
              finalResult: e.payload,
              // Replace live groups with final sorted results
              liveGroups: e.payload.groups || [],
            }));
          }),

          // Scan completion
          await listen<ScanComplete>('scan-complete', (e) => {
            update((state) => ({
              ...state,
              isScanning: false,
              phase: 'complete',
              scanComplete: e.payload,
            }));
          }),

          // Error handling
          await listen<ScanErrorEvent>('scan-error', (e) => {
            update((state) => ({
              ...state,
              isScanning: false,
              phase: 'error',
              error: e.payload.error,
            }));
          }),
        ];

        isInitialized = true;
      } catch (error) {
        console.error('Failed to initialize scanStore listeners:', error);
        throw error;
      }
    },

    // Start a new scan
    startScan() {
      update(() => ({
        isScanning: true,
        phase: 'collecting',
        phaseMessage: 'Starting scan...',
        progress: null,
        detectionProgress: null,
        liveGroups: [],
        finalResult: null,
        scanComplete: null,
        error: null,
      }));
    },

    // Reset store state
    reset() {
      set({
        isScanning: false,
        phase: 'idle',
        phaseMessage: '',
        progress: null,
        detectionProgress: null,
        liveGroups: [],
        finalResult: null,
        scanComplete: null,
        error: null,
      });
    },

    // Cleanup listeners (call on app unmount)
    cleanup() {
      if (unlisteners.length > 0) {
        unlisteners.forEach((fn) => fn());
        unlisteners = [];
        isInitialized = false;
      }
    },
  };
}

export const scanStore = createScanStore();

// Derived stores for convenience
export const isScanning = derived(scanStore, ($s) => $s.isScanning);
export const currentPhase = derived(scanStore, ($s) => $s.phase);

// Safe access to groups with fallback
export const duplicateGroups = derived(
  scanStore,
  ($s) => {
    // Prefer final results if available and valid
    if ($s.finalResult?.groups && Array.isArray($s.finalResult.groups)) {
      return $s.finalResult.groups;
    }
    // Fall back to live groups (already sorted)
    return $s.liveGroups || [];
  }
);

// Safe calculation of total wasted space
export const totalWastedSpace = derived(
  scanStore,
  ($s) => {
    // Prefer final calculation if available
    if ($s.finalResult?.total_wasted_space !== undefined) {
      return $s.finalResult.total_wasted_space;
    }
    // Calculate from live groups
    return ($s.liveGroups || []).reduce((sum, g) => sum + (g.wasted_space || 0), 0);
  }
);
