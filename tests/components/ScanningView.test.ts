import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ScanningView from '$lib/components/ScanningView.svelte';

const defaultProps = {
  phase: 'collecting' as const,
  progress: null,
  scanResult: null,
  onCancel: vi.fn(),
};

describe('ScanningView', () => {
  it('renders correct phase label for collecting', () => {
    render(ScanningView, { props: { ...defaultProps, phase: 'collecting' } });
    expect(screen.getByText('Scanning files...')).toBeInTheDocument();
  });

  it('renders correct phase label for partial_hashing', () => {
    render(ScanningView, { props: { ...defaultProps, phase: 'partial_hashing' } });
    expect(screen.getByText('Computing partial hashes...')).toBeInTheDocument();
  });

  it('renders correct phase label for full_hashing', () => {
    render(ScanningView, { props: { ...defaultProps, phase: 'full_hashing' } });
    expect(screen.getByText('Computing full hashes...')).toBeInTheDocument();
  });

  it('renders correct phase label for storing', () => {
    render(ScanningView, { props: { ...defaultProps, phase: 'storing' } });
    expect(screen.getByText('Analyzing duplicates...')).toBeInTheDocument();
  });

  it('renders correct phase label for complete', () => {
    render(ScanningView, { props: { ...defaultProps, phase: 'complete' } });
    expect(screen.getByText('Complete')).toBeInTheDocument();
  });

  it('shows file count and bytes from progress', () => {
    render(ScanningView, {
      props: {
        ...defaultProps,
        progress: {
          total_files: 1500,
          processed_files: 500,
          total_bytes: 1048576,
          current_path: null,
          skipped_files: 0,
          estimated_total: null,
        },
      },
    });
    // 1,500 files • 1 MB
    expect(screen.getByText(/1,500 files/)).toBeInTheDocument();
    expect(screen.getByText(/1 MB/)).toBeInTheDocument();
  });

  it('cancel button calls onCancel', () => {
    const onCancel = vi.fn();
    render(ScanningView, { props: { ...defaultProps, onCancel } });
    screen.getByText('Cancel').click();
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it('shows duration from scanResult', () => {
    render(ScanningView, {
      props: {
        ...defaultProps,
        progress: {
          total_files: 100,
          processed_files: 100,
          total_bytes: 5000,
          current_path: null,
          skipped_files: 0,
          estimated_total: null,
        },
        scanResult: {
          session_id: 1,
          total_files: 100,
          total_bytes: 5000,
          duplicate_groups: 3,
          duplicate_files: 6,
          wasted_space: 2000,
          duration_ms: 2500,
        },
      },
    });
    expect(screen.getByText(/2\.5s/)).toBeInTheDocument();
  });
});
