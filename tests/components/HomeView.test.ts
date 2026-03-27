import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import HomeView from '$lib/components/HomeView.svelte';
import type { DetectionResult } from '$lib/types';
import { makeGroup, makeResult } from '../factories';

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
    const result = makeResult([makeGroup(1, 100, 2)]);
    render(HomeView, { props: { ...defaultProps, detectionResult: result } });
    expect(screen.getByText('View Previous Results (1 group)')).toBeInTheDocument();
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
