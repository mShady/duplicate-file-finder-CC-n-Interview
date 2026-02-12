import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  ScanProgress,
  ScanComplete,
  DetectionResult,
  ScanPhaseEvent,
  ScanErrorEvent,
} from '$lib/types';

export type ScanPhase = ScanPhaseEvent['phase'] | 'idle';

// Module-level reactive state (Svelte 5 runes)
let _isScanning = $state(false);
let _phase = $state<ScanPhase>('idle');
let _progress = $state<ScanProgress | null>(null);
let _scanResult = $state<ScanComplete | null>(null);
let _detectionResult = $state<DetectionResult | null>(null);
let _error = $state<string | null>(null);

let unlisteners: UnlistenFn[] = [];
let initialized = false;

// Callbacks for events that need to trigger navigation changes in the UI layer
type ScanCallbacks = {
  onComplete?: () => void;
  onError?: () => void;
};

let callbacks: ScanCallbacks = {};

export const scanStore = {
  // Reactive getters
  get isScanning() { return _isScanning; },
  get phase() { return _phase; },
  get progress() { return _progress; },
  get scanResult() { return _scanResult; },
  get detectionResult() { return _detectionResult; },
  set detectionResult(value: DetectionResult | null) { _detectionResult = value; },
  get error() { return _error; },
  set error(value: string | null) { _error = value; },

  async init(cbs: ScanCallbacks = {}) {
    if (initialized) return;
    callbacks = cbs;

    unlisteners = [
      await listen<ScanProgress>('scan-progress', (e) => {
        _progress = e.payload;
      }),
      await listen<ScanComplete>('scan-complete', (e) => {
        _scanResult = e.payload;
        _isScanning = false;
        _phase = 'complete';
        callbacks.onComplete?.();
      }),
      await listen<DetectionResult>('scan-results', (e) => {
        _detectionResult = e.payload;
      }),
      await listen<ScanPhaseEvent>('scan-phase', (e) => {
        _phase = e.payload.phase;
      }),
      await listen<ScanErrorEvent>('scan-error', (e) => {
        _error = e.payload.error;
        _isScanning = false;
        _phase = 'idle';
        callbacks.onError?.();
      }),
    ];

    initialized = true;
  },

  startScan() {
    _isScanning = true;
    _phase = 'collecting';
    _detectionResult = null;
    _scanResult = null;
    _progress = null;
    _error = null;
  },

  cancelledScan() {
    _isScanning = false;
    _phase = 'idle';
  },

  handleScanError(msg: string) {
    _error = msg;
    _isScanning = false;
    _phase = 'idle';
  },

  reset() {
    _isScanning = false;
    _phase = 'idle';
    _progress = null;
    _scanResult = null;
    _detectionResult = null;
    _error = null;
  },

  cleanup() {
    unlisteners.forEach((fn) => fn());
    unlisteners = [];
    initialized = false;
    callbacks = {};
  },
};
