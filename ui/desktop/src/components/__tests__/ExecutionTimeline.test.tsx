import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import ExecutionTimeline from '../ExecutionTimeline';

describe('ExecutionTimeline', () => {
  it('renders Inspection Analysis Specialist with appropriate tag and badge', () => {
    render(
      <ExecutionTimeline
        specialistName="Inspection Analysis Specialist"
        operation="INSPECTION_ANALYSIS"
      />
    );

    expect(screen.getByText('Inspection Analysis Specialist')).toBeInTheDocument();
    expect(screen.getByText('NDT / Plant Integrity')).toBeInTheDocument();
  });

  it('renders General Chat Specialist default', () => {
    render(
      <ExecutionTimeline
        specialistName="General Chat Specialist"
        operation="GENERAL_CHAT"
      />
    );

    expect(screen.getByText('General Chat Specialist')).toBeInTheDocument();
    expect(screen.getByText('General Reasoning')).toBeInTheDocument();
  });

  it('expands execution flow on click to show phase status', async () => {
    const user = userEvent.setup();
    render(
      <ExecutionTimeline
        specialistName="Inspection Analysis Specialist"
        operation="INSPECTION_ANALYSIS"
        hasTools={true}
      />
    );

    // Toggle button
    const toggleButton = screen.getByRole('button', { name: /orchestrator route/i });
    await user.click(toggleButton);

    expect(screen.getByText('Execution Flow')).toBeInTheDocument();
    expect(screen.getByText('Understanding request')).toBeInTheDocument();
    expect(screen.getByText(/Specialist: Inspection Analysis Specialist/)).toBeInTheDocument();
    expect(screen.getByText('Executing tools')).toBeInTheDocument();
    expect(screen.getByText('Complete')).toBeInTheDocument();
  });
});
