# VPN y conexión a base de datos local

## Problema

Cuando la VPN está activa (NordVPN u otra), la conexión a una base de datos
en la **red local** se vuelve extremadamente lenta o falla, aunque el PC esté
conectado por cable a dicha red.

## Causa

La VPN está en modo **full tunnel**: enruta **todo** el tráfico (incluyendo el
de la red local) a través del túnel VPN. Los paquetes hacia la base de datos
salen por la VPN → servidor VPN → red local (si acaso), añadiendo latencia o
perdiéndose.

La VPN no solo afecta tráfico a internet; modifica la **tabla de
enrutamiento** completa del sistema operativo.

## Solución

### 1. Split Tunneling (NordVPN - recomendado)

NordVPN tiene split tunneling nativo en Windows:

1. Abre NordVPN → **Ajustes** → **Split Tunneling**
2. Actívalo
3. Agrega la IP de la base de datos (o el programa que se conecta a ella)
   en la lista de **apps/IPs que no usan VPN**

Esto fuerza que el tráfico hacia esa IP/local vaya por la interfaz local en
lugar del túnel VPN.

### 2. Ruta estática (alternativa técnica)

Si el split tunneling no funciona o prefieres hacerlo manual:

```cmd
ping elefante
nslookup elefante
ipconfig
```

Obtén la IP de la base de datos y el gateway local, luego:

```cmd
route add <IP_DE_LA_DB> mask 255.255.255.255 <GATEWAY_LOCAL>
```

### 3. Linux / Mac

```bash
sudo ip route add <IP_DE_LA_DB> via <GATEWAY_LOCAL> dev <INTERFAZ_LOCAL>
```

## Notas

- Si no conoces la IP de la base de datos, usa `ping <nombre_host>` o
  `nslookup <nombre_host>` desde un equipo que sí tenga acceso a esa red.
- El split tunneling de NordVPN es la solución más limpia en Windows.
