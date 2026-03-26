import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import SmartSelectionPanel from '$lib/components/SmartSelectionPanel.svelte';
import type { DuplicateGroup } from '$lib/types';

function makeGroup(id: number, paths: string[]): DuplicateGroup {
  return {
    id,
    hash: `hash_${id}`,
    file_size: 1024,
    files: paths.map((p, i) => ({
      path: p,
      size: 1024,
      created_at: 1000 + i,
      modified_at: 2000 + i,
      is_original: i === 0,
    })),
    wasted_space: 1024 * (paths.length - 1),
  };
}

const defaultProps = {
  groups: [makeGroup(1, ['/a.txt', '/b.txt'])],
  selectedFiles: new Set<string>(),
  onSelectionChange: vi.fn(),
};

describe('SmartSelectionPanel', () => {
  it('renders all four strategy buttons', () => {
    render(SmartSelectionPanel, { props: { ...defaultProps } });
    expect(screen.getByText('Select All Except Oldest')).toBeInTheDocument();
    expect(screen.getByText('Select Deepest Files')).toBeInTheDocument();
    expect(screen.getByText('Select by Depth')).toBeInTheDocument();
    expect(screen.getByText('Select by Location')).toBeInTheDocument();
  });

  it('Select by Location button is disabled when folder path is empty', () => {
    render(SmartSelectionPanel, { props: { ...defaultProps } });
    expect(screen.getByText('Select by Location')).toBeDisabled();
  });

  it('clicking Select All Except Oldest calls onSelectionChange', () => {
    const onSelectionChange = vi.fn();
    render(SmartSelectionPanel, { props: { ...defaultProps, onSelectionChange } });
    screen.getByText('Select All Except Oldest').click();
    expect(onSelectionChange).toHaveBeenCalledOnce();
  });

  it('clicking Select Deepest Files calls onSelectionChange', () => {
    const onSelectionChange = vi.fn();
    render(SmartSelectionPanel, { props: { ...defaultProps, onSelectionChange } });
    screen.getByText('Select Deepest Files').click();
    expect(onSelectionChange).toHaveBeenCalledOnce();
  });
});
