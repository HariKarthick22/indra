import type { ReactNode } from 'react';

export type IndraRailDestination = 'work' | 'memory' | 'sources' | 'trace' | 'sovereignty';

export interface IndraRailProps {
  active: IndraRailDestination;
  onSelect: (destination: IndraRailDestination) => void;
}

interface RailItem {
  id: IndraRailDestination;
  label: string;
  icon: ReactNode;
}

// Simple inline SVG placeholders — one distinct shape per destination.
// Real icon art is a separate design task.
const RAIL_ITEMS: RailItem[] = [
  {
    id: 'work',
    label: 'Work',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <circle cx="8" cy="8" r="5" fill="currentColor" />
      </svg>
    ),
  },
  {
    id: 'memory',
    label: 'Memory',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <rect x="3" y="3" width="10" height="10" fill="currentColor" />
      </svg>
    ),
  },
  {
    id: 'sources',
    label: 'Sources',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <polygon points="8,2 14,8 8,14 2,8" fill="currentColor" />
      </svg>
    ),
  },
  {
    id: 'trace',
    label: 'Trace',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <rect x="2" y="2" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="2" />
      </svg>
    ),
  },
  {
    id: 'sovereignty',
    label: 'Sovereignty',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M8 1 L14 4 V8 C14 12 11 14.5 8 15 C5 14.5 2 12 2 8 V4 Z" fill="currentColor" />
      </svg>
    ),
  },
];

const PROFILE_ICON = (
  <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
    <circle cx="8" cy="5.5" r="3" fill="currentColor" />
    <path d="M2 15c0-3.3 2.7-6 6-6s6 2.7 6 6" fill="currentColor" />
  </svg>
);

export function IndraRail({ active, onSelect }: IndraRailProps) {
  return (
    <nav
      aria-label="Primary"
      style={{
        width: 48,
        flexShrink: 0,
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'space-between',
        background: 'var(--surface)',
        borderRight: '1px solid var(--line)',
      }}
    >
      <ul
        style={{
          listStyle: 'none',
          margin: 0,
          padding: 0,
          width: '100%',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'stretch',
        }}
      >
        {RAIL_ITEMS.map((item) => {
          const isActive = item.id === active;
          return (
            <li key={item.id}>
              <button
                type="button"
                aria-label={item.label}
                aria-current={isActive ? 'page' : undefined}
                onClick={() => onSelect(item.id)}
                className={isActive ? 'indra-rail-item indra-rail-item--active' : 'indra-rail-item'}
                style={{
                  width: '100%',
                  height: 48,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  background: 'transparent',
                  border: 'none',
                  borderLeftWidth: '2px',
                  borderLeftStyle: 'solid',
                  borderLeftColor: isActive ? 'var(--focus)' : 'transparent',
                  color: isActive ? 'var(--text-hi)' : 'var(--text-dim)',
                  cursor: 'pointer',
                  transition: `color var(--m-ui) var(--ease-ui), border-color var(--m-ui) var(--ease-ui)`,
                }}
              >
                {item.icon}
              </button>
            </li>
          );
        })}
      </ul>
      <button
        type="button"
        aria-label="Profile"
        style={{
          width: '100%',
          height: 48,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'transparent',
          border: 'none',
          borderTop: '1px solid var(--line)',
          color: 'var(--text-dim)',
          cursor: 'pointer',
        }}
      >
        {PROFILE_ICON}
      </button>
    </nav>
  );
}
