import { existsSync } from 'fs';
import { join } from 'path';
import { execFileSync } from 'child_process';
import { get } from 'https';

const NORDVPN_EXE = 'nordvpn.exe';
const IP_CHECK_URL = 'https://api.ipify.org';
const IP_CHECK_TIMEOUT = 8000;
const RECONNECT_WAIT_MS = 3000;
const IP_POLL_INTERVAL_MS = 1500;
const IP_POLL_MAX_ATTEMPTS = 20;

// Códigos de país para NordVPN (2 letras, minúsculas)
// Variedad de países europeos y latinoamericanos para evitar patrones
const VPNPAISES = [
  'Spain',
  'France',
  'Germany',
  'Italy',
  'Portugal',
  'switzerland',
  'Belgium',
  'India',
  'Sweden', 
  'Ireland',
  'Brazil', 
  'Mexico', 
  'Albania', 
  'Mexico',
  'Poland',
  'United States',
  'South Korea',
  'Hong Kong',
  'Singapore',
  'Austria',
  'Norway'
];

function paisAleatorio(): string {
  return VPNPAISES[Math.floor(Math.random() * VPNPAISES.length)]!;
}

function resolveExe(nordvpnPath: string): string {
  return join(nordvpnPath, NORDVPN_EXE);
}

export function nordvpnDisponible(nordvpnPath: string): boolean {
  return existsSync(resolveExe(nordvpnPath));
}

async function obtenerIpPublica(): Promise<string | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), IP_CHECK_TIMEOUT);
    get(IP_CHECK_URL, (res) => {
      let data = '';
      res.on('data', (chunk) => { data += chunk; });
      res.on('end', () => {
        clearTimeout(timer);
        resolve(data.trim() || null);
      });
    }).on('error', () => {
      clearTimeout(timer);
      resolve(null);
    });
  });
}

async function esperarCambioIp(ipAnterior: string): Promise<void> {
  for (let i = 0; i < IP_POLL_MAX_ATTEMPTS; i++) {
    await new Promise(r => setTimeout(r, IP_POLL_INTERVAL_MS));
    const ipNueva = await obtenerIpPublica();
    if (ipNueva && ipNueva !== ipAnterior) return;
  }
}

export async function rotarVpn(nordvpnPath: string, onStatus?: (msg: string) => void): Promise<void> {
  const exe = resolveExe(nordvpnPath);

  onStatus?.('VPN: obteniendo IP actual...');
  const ipAnterior = await obtenerIpPublica();

  onStatus?.('VPN: desconectando...');
  try {
    execFileSync(exe, ['-d'], { timeout: 10000 });
  } catch {}

  await new Promise(r => setTimeout(r, RECONNECT_WAIT_MS));

  onStatus?.('VPN: conectando a nuevo servidor...');
  try {

    execFileSync(exe, ['-c', '-g', paisAleatorio()], { timeout: 15000 });
  } catch {}

  if (ipAnterior) {
    onStatus?.('VPN: esperando nueva IP...');
    await esperarCambioIp(ipAnterior);
  } else {
    await new Promise(r => setTimeout(r, RECONNECT_WAIT_MS));
  }

  onStatus?.('VPN: rotacion completada');
}
