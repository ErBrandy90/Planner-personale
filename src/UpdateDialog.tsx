import { useEffect, useRef, useState } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export function UpdateDialog({
  version,
  beforeInstall,
  onClose,
}: {
  version: string;
  beforeInstall: () => Promise<void>;
  onClose: () => void;
}) {
  const [phase, setPhase] = useState<'checking' | 'ready' | 'current' | 'error' | 'downloading' | 'installing'>(
    'checking',
  );
  const [update, setUpdate] = useState<Update | null>(null);
  const [message, setMessage] = useState('');
  const [progress, setProgress] = useState<number | null>(null);
  const resource = useRef<Update | null>(null);
  const alive = useRef(true);
  const busy = useRef(false);
  const checkNow = async () => {
    if (busy.current) return;
    busy.current = true;
    setPhase('checking');
    setMessage('');
    try {
      if (resource.current) {
        await resource.current.close();
        resource.current = null;
      }
      setUpdate(null);
      const found = await check({ timeout: 20000 });
      if (!alive.current) {
        await found?.close();
        return;
      }
      resource.current = found;
      setUpdate(found);
      setPhase(found ? 'ready' : 'current');
    } catch (e) {
      if (alive.current) {
        setPhase('error');
        setMessage(
          `Impossibile controllare gli aggiornamenti. Verifica la connessione e che il repository abbia una release pubblica con latest.json. Dettaglio: ${String(e)}`,
        );
      }
    } finally {
      busy.current = false;
    }
  };
  useEffect(() => {
    alive.current = true;
    void checkNow();
    return () => {
      alive.current = false;
      void resource.current?.close().catch(() => {});
    };
  }, []);
  const install = async () => {
    if (!update || busy.current) return;
    busy.current = true;
    setPhase('downloading');
    setProgress(null);
    let bytes = 0,
      total = 0;
    try {
      await update.download(
        (event) => {
          if (event.event === 'Started') total = event.data.contentLength ?? 0;
          if (event.event === 'Progress') {
            bytes += event.data.chunkLength;
            setProgress(total ? Math.min(100, Math.round((bytes / total) * 100)) : null);
          }
        },
        { timeout: 120000 },
      );
      setPhase('installing');
      await beforeInstall();
      await update.install({ restartAfterInstall: true });
      await relaunch();
    } catch (e) {
      setPhase('error');
      setMessage(`Aggiornamento non riuscito: ${String(e)}. Puoi riprovare il controllo.`);
    } finally {
      busy.current = false;
    }
  };
  const locked = phase === 'downloading' || phase === 'installing';
  useEffect(() => {
    const escape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !locked) onClose();
    };
    window.addEventListener('keydown', escape);
    return () => window.removeEventListener('keydown', escape);
  }, [locked, onClose]);
  return (
    <div className="modal-backdrop">
      <section className="modal" role="dialog" aria-modal="true" aria-labelledby="update-title">
        <h2 id="update-title">Aggiornamenti</h2>
        <p className="version">Versione installata: {version}</p>
        {phase === 'checking' && <p role="status">Controllo degli aggiornamenti…</p>}
        {phase === 'current' && <p role="status">Stai usando la versione più recente disponibile.</p>}
        {phase === 'ready' && update && (
          <>
            <p>
              È disponibile la versione <strong>{update.version}</strong>.
            </p>
            <pre className="release-notes">{update.body || 'Nessuna nota di rilascio disponibile.'}</pre>
            <p>L'app si chiuderà per aggiornarsi. Le modifiche saranno salvate e verrà creato un backup dei dati.</p>
          </>
        )}
        {phase === 'error' && <p role="alert">{message}</p>}
        {phase === 'downloading' && (
          <>
            <p role="status">Download dell'aggiornamento{progress === null ? '…' : `: ${progress}%`}</p>
            <progress max={100} value={progress ?? undefined} />
          </>
        )}
        {phase === 'installing' && <p role="status">Salvataggio, backup e installazione…</p>}
        <div className="dialog-actions">
          <button disabled={locked} onClick={onClose}>
            Chiudi
          </button>
          {phase === 'ready' && <button onClick={() => void install()}>Scarica e aggiorna</button>}
          {(phase === 'error' || phase === 'current') && (
            <button onClick={() => void checkNow()}>Riprova controllo</button>
          )}
        </div>
      </section>
    </div>
  );
}
