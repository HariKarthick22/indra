export interface IndraTopBarProps {
  sessionTitle: string;
  onSessionTitleChange: (title: string) => void;
  sessionBytes: number;
  budgetBytes: number;
  onToggleTheme: () => void;
}

function formatKb(bytes: number): string {
  return `${(bytes / 1000).toFixed(1)} kB`;
}

export function formatContextGauge(sessionBytes: number, budgetBytes: number): string {
  return `${formatKb(sessionBytes)} / ${formatKb(budgetBytes)}`;
}

export function IndraTopBar({
  sessionTitle,
  onSessionTitleChange,
  sessionBytes,
  budgetBytes,
  onToggleTheme,
}: IndraTopBarProps) {
  return (
    <header
      style={{
        height: 36,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 'var(--space-6)',
        padding: `0 ${'var(--space-6)'}`,
        background: 'var(--surface)',
        borderBottom: '1px solid var(--line)',
        color: 'var(--text)',
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--t-13)',
        lineHeight: 'var(--t-13--line-height)',
      }}
    >
      <input
        type="text"
        value={sessionTitle}
        onChange={(event) => onSessionTitleChange(event.target.value)}
        aria-label="Session title"
        style={{
          flex: '1 1 auto',
          minWidth: 0,
          background: 'transparent',
          border: 'none',
          outline: 'none',
          color: 'var(--text-hi)',
          fontFamily: 'var(--font-ui)',
          fontSize: 'var(--t-13)',
          fontWeight: 500,
        }}
      />
      <span
        aria-label="Context budget"
        style={{
          flexShrink: 0,
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--t-11)',
          lineHeight: 'var(--t-11--line-height)',
          color: 'var(--text-dim)',
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {formatContextGauge(sessionBytes, budgetBytes)}
      </span>
      <button
        type="button"
        onClick={onToggleTheme}
        aria-label="Toggle theme"
        style={{
          flexShrink: 0,
          width: 24,
          height: 24,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'transparent',
          border: 'none',
          borderRadius: 'var(--r-sm)',
          color: 'var(--text-dim)',
          cursor: 'pointer',
        }}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M8 2 A6 6 0 0 1 8 14 Z" fill="currentColor" />
        </svg>
      </button>
    </header>
  );
}
