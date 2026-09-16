import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { IndraShell } from './IndraShell';
import { formatContextGauge } from './IndraTopBar';

const DESTINATION_LABELS = ['Work', 'Memory', 'Sources', 'Trace', 'Sovereignty'];

function renderShell(overrides: Partial<React.ComponentProps<typeof IndraShell>> = {}) {
  const onSelect = vi.fn();
  const onSessionTitleChange = vi.fn();
  const onToggleTheme = vi.fn();

  const utils = render(
    <IndraShell
      active="work"
      onSelect={onSelect}
      sessionTitle="P-101 fit-for-service"
      onSessionTitleChange={onSessionTitleChange}
      sessionBytes={4100}
      budgetBytes={32000}
      onToggleTheme={onToggleTheme}
      {...overrides}
    >
      <p>working area</p>
    </IndraShell>
  );

  return { ...utils, onSelect, onSessionTitleChange, onToggleTheme };
}

describe('IndraShell', () => {
  it('renders all five rail destinations plus profile', () => {
    renderShell();

    for (const label of DESTINATION_LABELS) {
      expect(screen.getByRole('button', { name: label })).toBeInTheDocument();
    }
    expect(screen.getByRole('button', { name: 'Profile' })).toBeInTheDocument();
  });

  it('marks the active destination with the focus border and text-hi color', () => {
    renderShell({ active: 'memory' });

    const activeButton = screen.getByRole('button', { name: 'Memory' });
    expect(activeButton).toHaveAttribute('aria-current', 'page');
    expect(activeButton).toHaveStyle({
      borderLeftWidth: '2px',
      borderLeftStyle: 'solid',
      borderLeftColor: 'var(--focus)',
      color: 'var(--text-hi)',
    });

    const inactiveButton = screen.getByRole('button', { name: 'Work' });
    expect(inactiveButton).not.toHaveAttribute('aria-current');
    expect(inactiveButton).toHaveStyle({
      borderLeftWidth: '2px',
      borderLeftStyle: 'solid',
      // getComputedStyle normalizes the "transparent" keyword to its rgba
      // equivalent (true in real browsers too, not just jsdom).
      borderLeftColor: 'rgba(0, 0, 0, 0)',
    });
  });

  it('fires onSelect with the destination id when a rail icon is clicked', async () => {
    const user = userEvent.setup();
    const { onSelect } = renderShell();

    await user.click(screen.getByRole('button', { name: 'Sources' }));

    expect(onSelect).toHaveBeenCalledWith('sources');
  });

  it('renders the context gauge with the formatted byte string', () => {
    renderShell({ sessionBytes: 4100, budgetBytes: 32000 });
    expect(screen.getByLabelText('Context budget')).toHaveTextContent('4.1 kB / 32.0 kB');
  });

  it('renders a different formatted byte string for a second set of values', () => {
    renderShell({ sessionBytes: 512, budgetBytes: 16000 });
    expect(screen.getByLabelText('Context budget')).toHaveTextContent('0.5 kB / 16.0 kB');
  });
});

describe('formatContextGauge', () => {
  it('formats bytes as one-decimal kB, joined with a slash', () => {
    expect(formatContextGauge(4100, 32768)).toBe('4.1 kB / 32.8 kB');
    expect(formatContextGauge(0, 1000)).toBe('0.0 kB / 1.0 kB');
  });
});
