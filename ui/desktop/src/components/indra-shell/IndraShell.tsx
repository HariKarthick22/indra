import type { ReactNode } from 'react';
import { IndraRail, type IndraRailProps } from './IndraRail';
import { IndraTopBar, type IndraTopBarProps } from './IndraTopBar';

export interface IndraShellProps extends IndraRailProps, IndraTopBarProps {
  children: ReactNode;
}

export function IndraShell({
  active,
  onSelect,
  sessionTitle,
  onSessionTitleChange,
  sessionBytes,
  budgetBytes,
  onToggleTheme,
  children,
}: IndraShellProps) {
  return (
    <div
      style={{
        display: 'flex',
        width: '100%',
        height: '100%',
        background: 'var(--bg)',
        color: 'var(--text)',
        fontFamily: 'var(--font-ui)',
      }}
    >
      <IndraRail active={active} onSelect={onSelect} />
      <div
        style={{
          flex: '1 1 auto',
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          height: '100%',
        }}
      >
        <IndraTopBar
          sessionTitle={sessionTitle}
          onSessionTitleChange={onSessionTitleChange}
          sessionBytes={sessionBytes}
          budgetBytes={budgetBytes}
          onToggleTheme={onToggleTheme}
        />
        <main
          style={{
            flex: '1 1 auto',
            minHeight: 0,
            overflowY: 'auto',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <div
            style={{
              width: '100%',
              maxWidth: '78ch',
              padding: 'var(--space-7)',
            }}
          >
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
