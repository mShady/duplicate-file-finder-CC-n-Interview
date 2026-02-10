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

  return {
    subscribe,

    // Initialize event listeners
    async init() {
      // Clean up any existing listeners
      this.cleanup();

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
            liveGroups: e.payload.groups,
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

    // Cleanup listeners
    cleanup() {
      unlisteners.forEach((fn) => fn());
      unlisteners = [];
    },
  };
}

export const scanStore = createScanStore();

// Derived stores for convenience
export const isScanning = derived(scanStore, ($s) => $s.isScanning);
export const currentPhase = derived(scanStore, ($s) => $s.phase);
export const duplicateGroups = derived(
  scanStore,
  ($s) => $s.finalResult?.groups ?? $s.liveGroups,
);
export const totalWastedSpace = derived(
  scanStore,
  ($s) =>
    $s.finalResult?.total_wasted_space ??
    $s.liveGroups.reduce((sum, g) => sum + g.wasted_space, 0),
);
