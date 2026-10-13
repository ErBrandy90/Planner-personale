// Versione dell'app in un solo punto: package.json.
// tauri.conf.json la legge da lì ("version": "../package.json") e l'interfaccia la chiede a Tauri (getVersion).
// Cargo.toml, Cargo.lock e il manifest del connettore non possono leggerla da un altro file:
// questo script li allinea.
//
//   npm run versione -- 0.4.1     imposta la nuova versione ovunque
//   npm run versione              controlla che tutto coincida (usato dalla CI)
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const radice = fileURLToPath(new URL('..', import.meta.url));
const percorso = (file) => radice + file;
const leggi = (file) => readFileSync(percorso(file), 'utf8');
const scrivi = (file, testo) => writeFileSync(percorso(file), testo);

// Ogni file: come leggere la versione e come sostituirla, senza toccare il resto del contenuto.
const file = {
  'package.json': /^(\s*"version":\s*")([^"]+)(")/m,
  'package-lock.json': /^(\s*"version":\s*")([^"]+)(")/m,
  'src-tauri/Cargo.toml': /^(\[package\][^[]*?\nversion\s*=\s*")([^"]+)(")/m,
  'src-tauri/Cargo.lock': /^(name = "planner_settimanale"\r?\nversion = ")([^"]+)(")/m,
  'mcp-extension/manifest.json': /^(\s*"version":\s*")([^"]+)(")/m,
};
// package-lock.json ripete la versione anche nel pacchetto radice ("packages" > "")
const radiceLock = /("packages":\s*\{\s*"":\s*\{\s*"name":\s*"[^"]+",\s*"version":\s*")([^"]+)(")/;

function versioni() {
  const trovate = {};
  for (const [nome, regola] of Object.entries(file)) {
    const trovata = leggi(nome).match(regola);
    if (!trovata) throw new Error(`Versione non trovata in ${nome}`);
    trovate[nome] = trovata[2];
  }
  const lock = leggi('package-lock.json').match(radiceLock);
  if (!lock) throw new Error('Versione del pacchetto radice non trovata in package-lock.json');
  trovate['package-lock.json (radice)'] = lock[2];
  const conf = JSON.parse(leggi('src-tauri/tauri.conf.json'));
  if (conf.version !== '../package.json') throw new Error('tauri.conf.json deve avere "version": "../package.json"');
  return trovate;
}

const nuova = process.argv[2];
if (nuova) {
  if (!/^\d+\.\d+\.\d+$/.test(nuova)) {
    console.error(`Versione non valida: ${nuova} (usa il formato 1.2.3)`);
    process.exit(1);
  }
  for (const [nome, regola] of Object.entries(file)) scrivi(nome, leggi(nome).replace(regola, `$1${nuova}$3`));
  scrivi('package-lock.json', leggi('package-lock.json').replace(radiceLock, `$1${nuova}$3`));
  console.log(`Versione impostata a ${nuova}. Ricorda di aggiornare il changelog in src/App.tsx e la documentazione.`);
}

const trovate = versioni();
const diverse = new Set(Object.values(trovate));
if (diverse.size !== 1) {
  console.error('Versioni non allineate:');
  for (const [nome, versione] of Object.entries(trovate)) console.error(`  ${nome}: ${versione}`);
  console.error('Allinea con: npm run versione -- <versione>');
  process.exit(1);
}
console.log(`Versione ${[...diverse][0]} allineata in ${Object.keys(trovate).length} punti (+ tauri.conf.json da package.json).`);
