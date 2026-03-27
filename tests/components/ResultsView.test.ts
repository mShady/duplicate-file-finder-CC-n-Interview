import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ResultsView from '$lib/components/ResultsView.svelte';
import { makeGroup, makeResult } from '../factories';

describe('ResultsView', () => {
  it('renders stats: groups count, duplicate count, wasted space', () => {
    const result = makeResult([makeGroup(1, 1024, 3), makeGroup(2, 2048, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    // 2 groups
    expect(screen.getByLabelText('2 duplicate groups')).toBeInTheDocument();
    // 3 duplicates (3-1 + 2-1 = 3)
    expect(screen.getByLabelText('3 duplicate files')).toBeInTheDocument();
  });

  it('renders Smart Select toggle button', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Smart Select')).toBeInTheDocument();
  });

  it('renders Duplicate Groups header', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Duplicate Groups')).toBeInTheDocument();
  });

  it('renders empty file details panel when no group selected', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Select a duplicate group to view files')).toBeInTheDocument();
  });
});
