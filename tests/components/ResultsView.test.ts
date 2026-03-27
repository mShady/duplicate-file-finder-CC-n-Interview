import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import ResultsView from '$lib/components/ResultsView.svelte';
import { makeGroup, makeResult } from '../factories';

describe('ResultsView', () => {
  it('renders stats: groups count, duplicate count, wasted space', () => {
    const result = makeResult([makeGroup(1, 1024, 3), makeGroup(2, 2048, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    expect(screen.getByLabelText('2 duplicate groups')).toBeInTheDocument();
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

  it('clicking a group shows file details panel', async () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    // Click the group option
    screen.getByRole('option').click();

    await waitFor(() => {
      // FileDetailsPanel should now show file count heading
      expect(screen.getByText('2 Files')).toBeInTheDocument();
    });
  });

  it('toggling a file checkbox shows Delete Selected button', async () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    // Select a group first
    screen.getByRole('option').click();
    await waitFor(() => {
      expect(screen.getByText('2 Files')).toBeInTheDocument();
    });

    // Toggle the non-original file checkbox (second checkbox, first is disabled/original)
    const checkboxes = screen.getAllByRole('checkbox');
    const nonOriginalCheckbox = checkboxes.find((cb) => !cb.hasAttribute('disabled'));
    nonOriginalCheckbox!.click();

    await waitFor(() => {
      expect(screen.getByText('Delete Selected')).toBeInTheDocument();
      expect(screen.getByText(/1 files selected/)).toBeInTheDocument();
    });
  });

  it('Delete Selected calls onDeleteSelected with selected paths', async () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    const onDeleteSelected = vi.fn();
    render(ResultsView, { props: { result, onDeleteSelected } });

    // Select group → toggle file → click Delete Selected
    screen.getByRole('option').click();
    await waitFor(() => {
      expect(screen.getByText('2 Files')).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole('checkbox');
    const nonOriginalCheckbox = checkboxes.find((cb) => !cb.hasAttribute('disabled'));
    nonOriginalCheckbox!.click();

    await waitFor(() => {
      expect(screen.getByText('Delete Selected')).toBeInTheDocument();
    });

    screen.getByText('Delete Selected').click();
    expect(onDeleteSelected).toHaveBeenCalledOnce();
    expect(onDeleteSelected).toHaveBeenCalledWith(
      expect.arrayContaining([expect.stringContaining('file')])
    );
  });

  it('Clear Selection removes selected files', async () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    // Select group → toggle file
    screen.getByRole('option').click();
    await waitFor(() => {
      expect(screen.getByText('2 Files')).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByRole('checkbox');
    const nonOriginalCheckbox = checkboxes.find((cb) => !cb.hasAttribute('disabled'));
    nonOriginalCheckbox!.click();

    await waitFor(() => {
      expect(screen.getByText('Clear Selection')).toBeInTheDocument();
    });

    screen.getByLabelText('Clear all selected files').click();

    await waitFor(() => {
      expect(screen.queryByText('Delete Selected')).not.toBeInTheDocument();
    });
  });
});
