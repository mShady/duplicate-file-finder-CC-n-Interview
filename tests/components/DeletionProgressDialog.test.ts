import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import DeletionProgressDialog from '$lib/components/DeletionProgressDialog.svelte';

describe('DeletionProgressDialog', () => {
  it('shows Preparing phase when progress is null', () => {
    render(DeletionProgressDialog, { props: { progress: null } });
    expect(screen.getByText('Preparing...')).toBeInTheDocument();
    expect(screen.getByText('Preparing files for deletion...')).toBeInTheDocument();
  });

  it('shows Verifying phase label and progress count', () => {
    render(DeletionProgressDialog, {
      props: {
        progress: { phase: 'verifying', completed: 3, total: 10, current_path: null },
      },
    });
    expect(screen.getByText('Verifying files...')).toBeInTheDocument();
    expect(screen.getByText('3 of 10 files verified')).toBeInTheDocument();
  });

  it('shows Moving to Trash phase label', () => {
    render(DeletionProgressDialog, {
      props: {
        progress: { phase: 'trashing', completed: 5, total: 10, current_path: null },
      },
    });
    expect(screen.getByText('Moving to Trash...')).toBeInTheDocument();
    expect(screen.getByText('Moving 10 files to Trash...')).toBeInTheDocument();
  });
});
