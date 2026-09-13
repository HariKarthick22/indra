import React, { memo, useState } from 'react';
import {
  CheckCircle2,
  Loader2,
  ChevronRight,
  Cpu,
  Binary,
  Layers,
  FileSearch,
} from 'lucide-react';
import { cn } from '../utils';

export interface ExecutionTimelineProps {
  specialistName: string;
  operation?: string;
  hasTools?: boolean;
  isStreaming?: boolean;
  className?: string;
}

export function ExecutionTimeline({
  specialistName,
  operation,
  hasTools = false,
  isStreaming = false,
  className,
}: ExecutionTimelineProps) {
  const [expanded, setExpanded] = useState(false);

  // Specialist badge styling & icon based on domain
  const getSpecialistMeta = (name: string, op?: string) => {
    const lowerName = name.toLowerCase();
    const lowerOp = op?.toLowerCase() || '';

    if (lowerName.includes('inspection') || lowerOp.includes('inspection')) {
      return {
        badgeColor:
          'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/30 hover:bg-amber-500/15',
        icon: FileSearch,
        label: 'Inspection Analysis Specialist',
        tag: 'NDT / Plant Integrity',
      };
    }
    if (lowerName.includes('p&id') || lowerOp.includes('pid')) {
      return {
        badgeColor:
          'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/30 hover:bg-indigo-500/15',
        icon: Layers,
        label: 'P&ID Analysis Specialist',
        tag: 'Process Diagrams',
      };
    }
    if (lowerName.includes('engineering') || lowerOp.includes('engineering')) {
      return {
        badgeColor:
          'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/30 hover:bg-purple-500/15',
        icon: Binary,
        label: 'Engineering Calculation Specialist',
        tag: 'Deterministic Solver',
      };
    }
    if (lowerName.includes('code') || lowerOp.includes('code')) {
      return {
        badgeColor:
          'bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/30 hover:bg-cyan-500/15',
        icon: Cpu,
        label: 'Code Specialist',
        tag: 'Software & Scripts',
      };
    }
    return {
      badgeColor:
        'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/30 hover:bg-blue-500/15',
      icon: Cpu,
      label: name || 'General Chat Specialist',
      tag: 'General Reasoning',
    };
  };

  const meta = getSpecialistMeta(specialistName, operation);
  const IconComponent = meta.icon;

  // Timeline phases
  const phases = [
    {
      id: 'understanding',
      label: 'Understanding request',
      completed: true,
      inProgress: false,
    },
    {
      id: 'specialist',
      label: `Specialist: ${meta.label}`,
      completed: true,
      inProgress: false,
    },
    ...(hasTools
      ? [
          {
            id: 'tools',
            label: 'Executing tools',
            completed: !isStreaming,
            inProgress: isStreaming,
          },
        ]
      : []),
    {
      id: 'complete',
      label: isStreaming ? 'In progress...' : 'Complete',
      completed: !isStreaming,
      inProgress: isStreaming,
    },
  ];

  return (
    <div
      className={cn(
        'execution-timeline select-none mb-2.5 transition-all text-xs',
        className
      )}
      data-testid="execution-timeline"
    >
      {/* Header bar / Specialist badge */}
      <div className="flex items-center gap-2 flex-wrap">
        <div
          className={cn(
            'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border font-medium text-[11px] shadow-sm backdrop-blur-xs transition-colors',
            meta.badgeColor
          )}
        >
          <IconComponent className="w-3.5 h-3.5 shrink-0" />
          <span>{meta.label}</span>
          <span className="text-[10px] opacity-75 font-mono px-1 py-0.2 rounded bg-black/5 dark:bg-white/10">
            {meta.tag}
          </span>
        </div>

        {/* Quick Stepper Bar */}
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md text-text-secondary hover:text-text-primary hover:bg-bg-subtle transition-colors cursor-pointer"
          title="Click to toggle execution timeline"
        >
          <span className="text-[11px] font-mono text-text-subtle">
            Orchestrator Route
          </span>
          <ChevronRight
            className={cn(
              'w-3 h-3 text-text-secondary transition-transform duration-200',
              expanded && 'rotate-90'
            )}
          />
        </button>
      </div>

      {/* Expanded Phase Stepper */}
      {expanded && (
        <div className="mt-2 p-2.5 rounded-lg border border-border-subtle bg-bg-subtle/50 backdrop-blur-xs animate-in fade-in slide-in-from-top-1 duration-150">
          <div className="text-[11px] font-semibold text-text-secondary uppercase tracking-wider mb-2">
            Execution Flow
          </div>
          <div className="flex flex-col sm:flex-row items-start sm:items-center gap-1.5 sm:gap-2 text-[11px]">
            {phases.map((phase, idx) => (
              <React.Fragment key={phase.id}>
                <div className="flex items-center gap-1.5">
                  {phase.completed ? (
                    <CheckCircle2 className="w-3.5 h-3.5 text-green-600 dark:text-green-400 shrink-0" />
                  ) : phase.inProgress ? (
                    <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin shrink-0" />
                  ) : (
                    <div className="w-3.5 h-3.5 rounded-full border border-border-subtle shrink-0" />
                  )}
                  <span
                    className={cn(
                      phase.completed
                        ? 'text-text-primary font-medium'
                        : phase.inProgress
                        ? 'text-blue-600 dark:text-blue-400 font-medium'
                        : 'text-text-subtle'
                    )}
                  >
                    {phase.label}
                  </span>
                </div>
                {idx < phases.length - 1 && (
                  <ChevronRight className="hidden sm:inline w-3 h-3 text-text-subtle shrink-0" />
                )}
              </React.Fragment>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default memo(ExecutionTimeline);
