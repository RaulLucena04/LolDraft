# 11 - Instalación y actualizaciones

La app se distribuye como instalador de escritorio (Tauri v2 + NSIS) y se
actualiza desde un **canal de archivos**: un `latest.json` junto al instalador.
Por defecto el canal es la última release pública de GitHub; también admite una
carpeta local o cualquier URL. No hay firma de actualizaciones. Este documento
explica qué hace cada pieza, cómo publicar una versión, cómo dársela a otra
persona y qué límites tiene.

## 1. Compilar el instalador

```powershell
npm install
npm run app:build
```

`tauri build` compila Rust en modo `release`, construye la UI y genera:

| Artefacto | Ruta |
| --- | --- |
| Instalador NSIS | `target/release/bundle/nsis/LolDraft_<versión>_x64-setup.exe` |
| Instalador MSI | `target/release/bundle/msi/LolDraft_<versión>_x64_en-US.msi` |
| Ejecutable | `target/release/loldraft.exe` |

Requisitos en el equipo de destino: Windows 10/11 x64 y WebView2 Runtime
(Windows 11 lo incluye). El instalador NSIS es **por usuario**: instala en
`%LOCALAPPDATA%\LolDraft`, no pide administrador y deja un desinstalador.

El instalador **no está firmado**. Si se copia a otro equipo, Windows puede
mostrar SmartScreen o pedir confirmación; es esperado. Con *Más información →
Ejecutar de todas formas* se instala igual. Firmarlo no está previsto por ahora:
exigiría un certificado de firma de código (OV/EV) o Azure Trusted Signing.

## 2. Publicar una actualización

```powershell
npm run release                              # sube el parche: 0.1.0 -> 0.1.1
npm run release -- --github                  # además publica la release en GitHub
npm run release -- --version 0.2.0           # versión explícita
npm run release -- --no-bump                 # recompila sin cambiar la versión
npm run release -- --skip-build              # publica el último bundle compilado
npm run release -- --notes "Arregla el reparto de roles" --version 0.1.2
```

El script `scripts/release.mjs`:

1. Actualiza la versión en `package.json`, `apps/desktop/package.json`,
   `package-lock.json`, `src-tauri/tauri.conf.json` y `[workspace.package]` de
   `Cargo.toml` (`Cargo.lock` se refresca al compilar). La versión que muestra la
   app sale de `CARGO_PKG_VERSION`, así que todas deben coincidir.
2. Ejecuta `npm run app:build` (salvo `--skip-build`).
3. Copia el instalador NSIS a `%APPDATA%\com.loldraft.desktop\updates`.
4. Escribe `latest.json` con la versión, el nombre del instalador, las notas y
   el `sha256` del ejecutable.
5. Con `--github`, crea la release `v<versión>` con `gh` y sube el instalador y
   `latest.json` como assets. Si la release ya existe, sube los assets de nuevo
   y actualiza las notas (`--clobber`).

`--github` necesita [GitHub CLI](https://cli.github.com/) con sesión iniciada
(`gh auth login`, o la variable `GH_TOKEN`). El repositorio se detecta solo;
`--repo owner/nombre` lo cambia, por ejemplo para publicar en un repositorio de
releases separado del código.

Por defecto **nunca reutiliza la misma versión sin avisar**: si no indicas
`--version` sube el parche, porque la app solo ofrece instalar versiones
estrictamente más nuevas. `--no-bump` existe para reconstruir un instalador o
para la primera publicación.

## 3. La tarjeta de actualizaciones

Inicio → columna derecha → **Aplicación · Versión X**.

1. **Origen** (opcional): carpeta o URL base. Vacío usa el canal público
   `https://github.com/RaulLucena04/LolDraft/releases/latest/download`, es decir,
   los assets de la última release. Se guarda en el ajuste `updates.source`.
   Para probar una compilación sin publicarla, indica la carpeta
   `%APPDATA%\com.loldraft.desktop\updates`.
2. **Buscar actualizaciones**: lee `<origen>/latest.json`, compara versiones y
   muestra el canal y la ruta exacta consultada.
3. **Instalar**: solo aparece si la versión del canal es mayor.
   - **Canal local**: ejecuta el instalador y muestra su ruta. El instalador
     pide cerrar LolDraft si sigue abierto.
   - **Canal HTTP** (incluido GitHub): abre la descarga en el navegador. La app
     **no** descarga ni ejecuta nada de internet por su cuenta.

La comparación es numérica (`0.2.0 > 0.1.9`, `1.0 > 0.9.9`) e ignora etiquetas
de preestreno: `0.2.0-beta` no se considera más nueva que `0.2.0`.

## 4. Formato de `latest.json`

```json
{
  "version": "0.2.0",
  "installer": "LolDraft_0.2.0_x64-setup.exe",
  "notes": "Arregla el reparto de roles en SoloQ.",
  "sha256": "…",
  "generated_at": "2026-11-18T18:00:00.000Z"
}
```

| Campo | Obligatorio | Uso |
| --- | --- | --- |
| `version` | sí | Versión publicada; se compara con la instalada. |
| `installer` | sí | Nombre relativo del instalador dentro del canal. |
| `notes` | no | Texto que se muestra antes de instalar. |
| `sha256` | no | Solo informativo; la app **no** lo verifica. |
| `generated_at` | no | Metadatos del script de publicación. |

El `installer` debe ser un nombre simple (`setup.exe`) o una subcarpeta
(`v0.2/setup.exe`), con letras, números, punto, guion o guion bajo. Se rechazan
rutas absolutas, `..` y nombres con otros caracteres: el manifiesto no puede
apuntar a un ejecutable arbitrario del sistema.

## 5. El canal público de GitHub

La app consulta, sin configurar nada:

```
https://github.com/RaulLucena04/LolDraft/releases/latest/download/latest.json
```

`releases/latest/download/<nombre>` sirve cada asset de la última release no
marcada como preestreno, por eso el instalador y `latest.json` deben estar en la
misma release. El `installer` del manifiesto se resuelve como
`…/latest/download/<installer>`, así que el nombre del asset debe coincidir con
el del fichero.

Para que el botón funcione con otra persona:

1. El repositorio (o al menos sus releases) debe ser **público**; si es privado,
   la URL devuelve 404 sin sesión.
2. La release debe ser la **última** y no estar marcada como preestreno. `gh
   release create --latest` lo deja así.
3. No borres ni renombres los assets de esa release: `latest.json` apunta al
   instalador por nombre.

GitHub cachea las redirecciones de `releases/latest` unos minutos. Si acabas de
publicar y la app sigue viendo la versión anterior, espera y vuelve a buscar.

Alternativa sin publicar el código: crea un repositorio público solo para las
releases (por ejemplo `LolDraft-releases`) y publica con
`--repo usuario/LolDraft-releases`. En ese caso indica esa URL en **Origen**, o
cambia `DEFAULT_CHANNEL` en `src-tauri/src/updates.rs`.

También vale cualquier hosting estático: copia el instalador y `latest.json` a
`https://tu-dominio/updates/` e indica esa URL como origen. La app consulta
`<url>/latest.json` con un timeout de 10 s y un límite de 64 KiB; la descarga la
abre el navegador.

## 6. Compartir la app con otra persona

Lo más simple es enviarle el instalador:

```
target\release\bundle\nsis\LolDraft_<versión>_x64-setup.exe
```

Se instala por usuario en `%LOCALAPPDATA%\LolDraft`, sin administrador, y crea
accesos en el menú Inicio y el escritorio. Al abrirla por primera vez solo hay
catálogo de campeones: para tener recomendaciones debe pulsar **Cargar piloto
inferido** en Inicio. Es un dato `Inferred`, no curado.

A partir de ahí, si el repositorio es público y hay releases, **Inicio →
Actualizaciones → Buscar actualizaciones** le dirá si hay una versión nueva y el
botón abrirá la descarga. Sin canal público, hay que enviarle cada instalador
nuevo a mano.

## 7. Seguridad y límites

- **Sin firma.** La app no verifica la autenticidad del instalador. Solo ejecuta
  un `.exe` que ya está en el canal local configurado; en HTTP abre el navegador
  y decide el usuario. El `sha256` del manifiesto es informativo. No apuntes el
  canal a una carpeta o URL que no controles.
- **Sin autoactualización.** Nunca se descarga ni se ejecuta en segundo plano.
- **Sin downgrade.** Una versión igual o anterior no se ofrece como instalable.
- **Sin actualizaciones delta.** Cada versión publica el instalador completo.
- **Solo Windows.** El script asume las rutas de `%APPDATA%` y el bundle NSIS.
- **El canal local es de este equipo.** Otra máquina necesita su propio
  instalador o el canal público de GitHub.
- **Depende de la red.** Sin conexión, comprobar el canal público falla con
  `update_unreachable`; el canal local sigue funcionando.
- **Errores estables:** `update_not_found` (no hay `latest.json`),
  `update_unreachable` (red o estado HTTP), `update_invalid` (manifiesto o ruta
  no válidos), `update_invalid_version`, `update_installer_missing` (el
  manifiesto apunta a un instalador que no está), `update_changed` (el canal
  cambió entre comprobar e instalar), `update_not_newer`, `update_install_failed`
  y `update_launch_failed`.

## 8. Verificación rápida

```powershell
# 1. Compilar e instalar en el equipo de pruebas
npm run app:build
target\release\bundle\nsis\LolDraft_0.1.0_x64-setup.exe

# 2. Probar el canal local antes de publicar
npm run release -- --version 0.1.1 --notes "Prueba de actualización"
#    y en la app: Origen = %APPDATA%\com.loldraft.desktop\updates

# 3. Publicar de verdad
npm run release -- --version 0.1.2 --notes "…" --github

# 4. En la app: Inicio -> Actualizaciones -> Buscar -> Descargar
```

El canal local queda en `%APPDATA%\com.loldraft.desktop\updates`. Para comprobar
el caso "al día", publica la misma versión instalada; para comprobar un manifiesto
roto, edita `latest.json` a mano y vuelve a buscar.
