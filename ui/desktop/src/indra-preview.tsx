import { useState } from 'react';
import ReactDOM from 'react-dom/client';
import { IndraShell, type IndraRailDestination } from './components/indra-shell';

function Preview() {
  const [active, setActive] = useState<IndraRailDestination>('work');
  const [title, setTitle] = useState('P-101 fit-for-service');

  return (
    <IndraShell
      active={active}
      onSelect={setActive}
      sessionTitle={title}
      onSessionTitleChange={setTitle}
      sessionBytes={4100}
      budgetBytes={32000}
      onToggleTheme={() => document.documentElement.classList.toggle('dark')}
    >
      <div style={{ fontSize: 14, lineHeight: 1.6 }}>
        <p>
          Active destination: <strong>{active}</strong>
        </p>
        <p>This is the working area — 78ch measure, centred, scrolls.</p>
        <p>
          Ask about P-101 is P-101 fit for service? Use the August NDT. The August NDT records 6.2
          mm at grid E-4 ⟦1⟧ against a minimum allowable of 7.1 mm ⟦2⟧.
        </p>
      </div>
    </IndraShell>
  );
}

document.documentElement.classList.add('dark');
ReactDOM.createRoot(document.getElementById('root')!).render(<Preview />);
