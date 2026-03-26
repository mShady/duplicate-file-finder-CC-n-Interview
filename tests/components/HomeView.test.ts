import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import HomeView from '$lib/components/HomeView.svelte';
import type { DetectionResult } from '$lib/types';

const defaultProps = {
  selectedPaths: [] as string[],
  isScanning: false,
  error: null as string | null,
  detectionResult: null as DetectionResult | null,
  onPathsChange: vi.fn(),
  onStartScan: vi.fn(),
  onViewResults: vi.fn(),
};

describe('HomeView', () => {
  it('renders heading', () => {
    render(HomeView, { props: { ...defaultProps } });
    expect(screen.getByText('Find Duplicate Files')).toBeInTheDocument();
  });

  it('shows error banner when error is set', () => {
    render(HomeView, { props: { ...defaultProps, error: 'Something went wrong' } });
    expect(screen.getByRole('alert')).toHaveTextContent('Something went wrong');
  });

  it('scan button disabled when no paths selected', () => {
    render(HomeView, { props: { ...defaultProps, selectedPaths: [] } });
    expect(screen.getByText('Start Scan')).toBeDisabled();
  });

  it('scan button disabled when scanning', () => {
    render(HomeView, {
      props: { ...defaultProps, selectedPaths: ['/tmp'], isScanning: true },
    });
    expect(screen.getByText('Start Scan')).toBeDisabled();
  });

  it('scan button enabled with paths and not scanning', () => {
    render(HomeView, {
      props: { ...defaultProps, selectedPaths: ['/tmp'], isScanning: false },
    });
    expect(screen.getByText('Start Scan')).not.toBeDisabled();
  });

  it('shows View Previous Results when detectionResult provided', () => {
    const result: DetectionResult = {
      groups: [
        {
          id: 1,
          hash: 'abc',
          file_size: 100,
          files: [],
          wasted_space: 0,
        },
      ],
      duplicate_count: 1,
      total_wasted_space: 100,
      unique_files: 1,
      stats: {
        size_groups: 0,
        size_candidates: 0,
        partial_hashes: 0,
        full_hashes: 0,
        size_grouping_ms: 0,
        partial_hashing_ms: 0,
        full_hashing_ms: 0,
      },
    };
    render(HomeView, { props: { ...defaultProps, detectionResult: result } });
    expect(screen.getByText('View Previous Results (1 groups)')).toBeInTheDocument();
  });

  it('calls onStartScan when button clicked', () => {
    const onStartScan = vi.fn();
    render(HomeView, {
      props: { ...defaultProps, selectedPaths: ['/tmp'], onStartScan },
    });
    screen.getByText('Start Scan').click();
    expect(onStartScan).toHaveBeenCalledOnce();
  });
});
