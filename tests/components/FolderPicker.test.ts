import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import { open } from '@tauri-apps/plugin-dialog';
import FolderPicker from '$lib/components/FolderPicker.svelte';

describe('FolderPicker', () => {
  it('renders empty state when no paths selected', () => {
    render(FolderPicker, { props: { selectedPaths: [], onPathsChange: vi.fn() } });
    expect(screen.getByText('No folders selected')).toBeInTheDocument();
  });

  it('shows path items when selectedPaths is non-empty', () => {
    render(FolderPicker, {
      props: { selectedPaths: ['/home/user/docs', '/tmp'], onPathsChange: vi.fn() },
    });
    expect(screen.getByText('/home/user/docs')).toBeInTheDocument();
    expect(screen.getByText('/tmp')).toBeInTheDocument();
  });

  it('remove button calls onPathsChange without that path', () => {
    const onPathsChange = vi.fn();
    render(FolderPicker, {
      props: { selectedPaths: ['/a', '/b'], onPathsChange },
    });
    const removeBtn = screen.getByLabelText('Remove /a');
    removeBtn.click();
    expect(onPathsChange).toHaveBeenCalledWith(['/b']);
  });

  it('Clear All calls onPathsChange with empty array', () => {
    const onPathsChange = vi.fn();
    render(FolderPicker, {
      props: { selectedPaths: ['/a', '/b'], onPathsChange },
    });
    screen.getByText('Clear All').click();
    expect(onPathsChange).toHaveBeenCalledWith([]);
  });

  it('Add Folder button calls open() from dialog plugin', () => {
    vi.mocked(open).mockResolvedValueOnce(null);
    render(FolderPicker, {
      props: { selectedPaths: [], onPathsChange: vi.fn() },
    });
    screen.getByText('Add Folder').click();
    expect(open).toHaveBeenCalledWith({
      directory: true,
      multiple: true,
      title: 'Select folders to scan',
    });
  });
});
