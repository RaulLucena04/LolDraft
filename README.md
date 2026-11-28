# LolDraft — Draft Assistant competitivo

Aplicación de escritorio para simular, analizar y asistir drafts competitivos de League of Legends.
Núcleo en Rust, escritorio con Tauri v2 y UI en React + TypeScript + Tailwind CSS.

> **Estado del repositorio:** fases 1–3 cerradas y fase 4 en curso, publicadas en
> [Releases](https://github.com/RaulLucena04/LolDraft/releases/latest) (v0.1.9). El simulador de 20
> acciones, el análisis de composición y las recomendaciones de pick/ban explicables funcionan; el
> modo SoloQ se sincroniza solo con el cliente de League of Legends, incluido tu propio pick. La
> Fase 4 importa estadísticas observadas (CSV/JSON), conecta `power` a las recomendaciones con
> separación estricta SoloQ/profesional y recomienda objetos y runas **por función** frente al rival:
> no hace falta importar builds para eso. El talón de Aquiles siguen siendo los datos: solo existe un
> piloto inferido de 38 campeones, etiquetado siempre como `Inferred`, y los datasets observados hay
> que importarlos a mano.

## Qué hace diferente a esta herramienta

No es un visor de winrates. El objetivo es que el motor evalúe la **composición completa** de ambos
equipos:

- qué arquetipos cubre y cuáles le faltan;
- qué amenazas plantea el rival y qué respuestas necesita el equipo;
- sinergias reales entre campeones y condiciones de victoria;
- el contexto del draft (primer pick, último pick, lado azul/rojo, información revelada);
- el coste de oportunidad y los riesgos de cada pick.

Cada recomendación deberá explicarse con las mismas contribuciones numéricas que produjeron la
puntuación. No habrá explicaciones generadas a posteriori ni un LLM inventando motivos.

## Qué funciona ahora

- **Simulador competitivo:** formato estándar de 20 acciones (6 bans, 6 picks, 4 bans, 4 picks),
  validación de turno, lado, tipo de acción y campeón duplicado.
- **Undo/redo por snapshots**, sin corromper el estado ni el historial. Al reabrir una sesión
  guardada, el historial de deshacer se reconstruye desde sus acciones.
- **Asignación óptima de roles** por permutaciones (hasta 5!), con detección de ambigüedad y
  override manual para flex picks.
- **Persistencia SQLite:** la sesión se guarda tras cada acción y se recupera al reabrir.
- **Catálogo Data Dragon 16.22.1:** 173 campeones, retratos locales y 171 con roles importados.
  `Locke` y `Zaahen` se muestran como «rol pendiente», sin inventar un rol.
- **Sincronización Data Dragon** desde la app para actualizar identidades sin borrar roles curados.
- **UI de escritorio:** inicio, estado de datos, sesiones recientes, simulador con paneles azul/rojo,
  timeline compacta de 20 turnos, selector de campeones visible en la columna central con búsqueda y
  filtros, drawer de campeón y avisos de error estables.
- **Actualizaciones sobre la app:** al arrancar se comprueba GitHub Releases; si hay una versión
  nueva, un aviso permite descargarla, verificar su `sha256` e instalarla en silencio, y LolDraft se
  reinicia con la versión nueva.
- **Datos honestos:** los perfiles estratégicos ausentes se muestran como pendientes; las celdas
  inferidas se etiquetan como `Inferred` y no se presentan como observadas. No se muestran ceros ni
  winrates falsos.
- **Modo SoloQ:** el draft se rellena solo desde el cliente de League of Legends cada dos segundos
  (con pausa y actualización manual), incluido tu pick cuando lo bloqueas. Mejor pick por turno con
  entrada manual como respaldo; al abrir el análisis una vez, el panel se recalcula solo cuando cambia
  el draft, sin volver a pulsar.
- **Estadísticas observadas:** importación de CSV/JSON con esquema validado, shrinkage y cuarentena de
  filas inválidas; el dataset activo alimenta `power` solo en su cola, y la UI muestra su procedencia
  (id, parche, fuente, licencia y filas). Sin dataset, la señal se marca ausente.
- **Objetos y runas por función:** dos paneles separados con iconos y nombres oficiales. Cada campeón
  parte de una plantilla curada (inicio, botas, núcleo, situacionales y runas) incluida en el binario;
  si no existe, se aplican reglas por función. La build se ajusta a las amenazas del rival (daño
  físico/mágico, CC, sustain, tanques…), exige coherencia de daño AD/AP y solo muestra objetos
  situacionales cuando el rival presenta la amenaza que contrarrestan. Explica cada sugerencia y
  etiqueta el encaje como plantilla/regla, nunca como winrate. No requiere importar JSON.
- **CounterStats:** los rivales leídos enlazan a su página pública de CounterStats. La app no copia ni
  redistribuye esas estadísticas; abre la fuente externa para quien quiera consultar el enfrentamiento.
- **Builds importadas (opcional):** dataset JSON por campeón, rol y cola para quien quiera una build
  observada. La app elige la de mejor winrate ajustado (o la más jugada / la curada, etiquetándolo) y
  muestra datos, procedencia y confianza. Ya no es necesaria para jugar SoloQ.

## Análisis estratégico (Fase 2)

- **Taxonomía v1.0.0** validada: 20 arquetipos, 18 traits y una rúbrica de intensidad de cinco anclas.
- **Esquema de perfiles** con procedencia (`Riot`, curado, importado o inferido), confianza por celda,
  distinción entre `None` (desconocido) y `0.0` (no aporta), validación de shares de daño y conjuntos
  versionados que se cargan enteros o se rechazan.
- **Persistencia SQLite** de conjuntos de perfiles: roles, arquetipos y traits versionados por
  conjunto, activación única e importación transaccional que no deja datos a medias.
- **Agregación de composiciones** con intervalos de desconocimiento, daño, curva de poder, rango, CC,
  movilidad, waveclear, objetivos, necesidades por suelos/amenazas/balance, condiciones de victoria,
  confianza y señales explicables.
- **Sinergias derivadas** entre pares del mismo equipo (engage + burst/DPS, peel + DPS, poke + asedio,
  catch + burst, etc.) con intervalo, confianza y una pareja como máximo por regla.
- **Lectura de líneas y ventajas de equipo** derivadas de perfiles: presión/all-in por rol compartido
  y herramientas ofensivas frente a la respuesta rival. No son estadísticas de enfrentamientos.
- **Piloto inferido importable:** 38 campeones, todos los roles, celdas declaradas con procedencia
  `Inferred` y confianza 0.35. Se importa y activa solo cuando el usuario lo pide desde la pantalla de
  inicio; la UI lo marca como «inferido, revisión pendiente».
- **Panel de análisis en el draft:** intervalos por arquetipo, reparto de daño, carencias confirmadas
  o posibles con sus motivos, condiciones de victoria, sinergias derivadas, lectura de líneas,
  ventajas de equipo, fortalezas y aviso de análisis parcial. No hay barras de winrate.
- **Detalle de campeón:** perfil del set activo con procedencia y confianza por celda, cobertura,
  daño y sinergias derivadas con los compañeros de su lado. Si el campeón no está en el set, lo dice
  y no rellena celdas.
- **Curación por fichero:** `loldraft-curate` expande propuestas, genera partes de revisión y
  convierte celdas `Inferred` conocidas en `Curated`. `edit` aplica valores humanos explícitos a
  celdas y roles; ninguna heurística rellena huecos. `diff` audita los cambios entre dos conjuntos
  antes de importarlos. Los conjuntos revisados se importan y activan desde la pantalla de inicio.
- **Pendiente:** revisión experta del piloto y de 20-30 composiciones, sinergias curadas y matchups
  observados. La herramienta de curación ya existe; falta el trabajo de revisión con criterio de
  experto.

## Recomendaciones de pick/ban (Fase 3)

- **Features explicables:** `need`, `answer`, `synergy`, `counter`, `power` (con dataset observado),
  `flex`, `balance` y `safety`, con renormalización de pesos cuando falta una señal, pesos por fase
  del draft, penalización por confianza y diversificación por rol y arquetipo.
- **Explicaciones con parámetros**, no texto generado: puntuación, contribuciones, riesgos, motivos,
  carencias que cubre y datos que faltan. La invariante `contribuciones + riesgos = puntuación` se
  cumple y se testea.
- **Baneos desde el plan del rival**, no solo por winrate.
- **UI:** panel de recomendaciones con detalle expandible, lista de alternativas y desglose; aviso
  explícito cuando los datos no bastan para recomendar.
- **Pendiente:** reasignación completa de roles al evaluar y filtros de alternativas.

Todavía **no** incluye: sinergias curadas ni matchups observados, importación de drafts profesionales
ni fearless draft. La potencia observada ya está conectada cuando importas un dataset.

## Descargar la app

Windows 10/11 x64. Instalador por usuario, sin administrador:

1. Descarga `LolDraft_<versión>_x64-setup.exe` desde
   [Releases](https://github.com/RaulLucena04/LolDraft/releases/latest).
2. El instalador **no está firmado**: SmartScreen mostrará un aviso → *Más información* →
   *Ejecutar de todas formas*.
3. Al abrir, pulsa **Cargar piloto inferido** en Inicio. Sin ese paso solo hay catálogo de campeones;
   el piloto es un dato `Inferred` (38 campeones) y la app lo marca como pendiente de revisión.
4. Las versiones siguientes se detectan con **Buscar actualizaciones**, que abre la descarga.

## Arranque rápido (Windows)

Requisitos:

1. Rust estable con toolchain MSVC (`stable-x86_64-pc-windows-msvc`).
2. Visual Studio Build Tools con *Desktop development with C++* (MSVC + Windows SDK).
3. Node.js 22 o superior.
4. WebView2 Runtime (Windows 11 lo incluye).

```powershell
# Desde la raíz del repositorio
npm install
npm run app:dev
```

`npm run app:dev` inicia Vite y la ventana de Tauri. Para compilar el instalador:

```powershell
npm run app:build
```

El instalador NSIS queda en `target/release/bundle/nsis/LolDraft_<versión>_x64-setup.exe`
(instalación por usuario, sin administrador). Ese `.exe` es lo que se comparte con
otra persona: se instala y crea accesos directos; no hace falta nada más.

La app incluye un botón **Buscar actualizaciones** en Inicio. Por defecto consulta
la última release pública de GitHub
(`https://github.com/RaulLucena04/LolDraft/releases/latest/download`), así que
para publicar una versión nueva:

```powershell
npm run release -- --github           # sube el parche, compila y publica en GitHub
npm run release -- --version 0.2.0 --github
```

Sin `--github`, el script solo deja el instalador y `latest.json` en
`%APPDATA%\com.loldraft.desktop\updates`, útil para probar el botón antes de
publicar (indica esa carpeta en **Origen**). `--github` requiere
[GitHub CLI](https://cli.github.com/) con sesión (`gh auth login`) y un
repositorio público.

El instalador **no está firmado**, así que SmartScreen puede pedir confirmación
en otro equipo. Detalles, formato del manifiesto, publicación y límites en
[11 — Instalación y actualizaciones](docs/11-instalacion-actualizaciones.md).

Si `cargo` no está en el `PATH` del shell:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Verificaciones

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run build:ui
```

La CI de GitHub Actions ejecuta exactamente estas comprobaciones en Windows.

## Estructura

| Ruta | Responsabilidad |
| --- | --- |
| `crates/loldraft-domain` | Dominio puro: formato, estado del draft, historial y roles. Sin I/O. |
| `crates/loldraft-engine` | Taxonomía, perfiles, propuestas, análisis de composiciones y recomendaciones de pick/ban. Sin I/O. |
| `crates/loldraft-data` | SQLite, migraciones, semillas y CRUD de sesiones/ajustes. |
| `crates/loldraft-sources` | Clientes de fuentes externas: Data Dragon y cliente local de League (LCU). |
| `crates/loldraft-curation` | CLI de curación por fichero: propuestas, revisión, aprobación y validación. |
| `src-tauri` | Estado de aplicación, DTOs, comandos Tauri y errores estables. |
| `apps/desktop` | UI React + TypeScript + Tailwind. |
| `crates/loldraft-data/seeds/champions.json` | Semilla reproducible de identidad y roles. |
| `apps/desktop/public/champions` | Retratos empaquetados de Data Dragon. |
| `apps/desktop/public/items` | Iconos de objetos empaquetados de Data Dragon. |
| `apps/desktop/public/runes` | Iconos de runas clave empaquetados de Data Dragon. |
| `apps/desktop/public/fonts` | Tipografía de títulos Cinzel (SIL OFL, licencia incluida). |
| `scripts/fetch-static-data.mjs` | Generador reproducible de la semilla, retratos, iconos y nombres oficiales del catálogo. |

### Comandos Tauri

`app_bootstrap`, `champions_list`, `data_status`, `data_sync_ddragon`, `draft_new`, `draft_action`,
`draft_undo`, `draft_redo`, `draft_set_role`, `draft_list`, `draft_load`, `draft_delete`,
`profiles_list`, `profiles_import_pilot`, `profiles_import_proposal`, `profiles_import_set`,
`profiles_activate`, `profile_champion`, `analysis_current`, `recommendations_current`,
`soloq_recommend`, `loadout_recommend`, `lcu_champ_select`, `stats_list`, `stats_import_json`,
`stats_import_csv`, `stats_activate`, `stats_delete`, `builds_list`, `builds_import_json`,
`builds_activate`, `builds_delete`, `builds_for`, `settings_get`, `settings_update`, `update_check`,
`update_install`, `open_external`.

La UI no recalcula reglas ni asignaciones: consume los DTOs que produce Rust.

## Datos y procedencia

- **Identidad y retratos:** [Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon) de
  Riot Games, parche 16.22.1. Los retratos están empaquetados para funcionar sin conexión.
- **Roles:** Meraki Analytics (`positions`), importados con `source: "meraki"` y confianza 0.8.
  La atribución se muestra en la UI. Los campeones sin dato quedan pendientes.
- **Tipografía de títulos:** [Cinzel](https://github.com/NDISCOVER/Cinzel) bajo licencia
  **SIL Open Font License 1.1**, empaquetada en `apps/desktop/public/fonts` junto a su licencia.
  Se usa solo para títulos y encabezados; el cuerpo mantiene la sans del sistema. No es un activo
  de Riot ni sustituye a la tipografía del cliente.
- **Perfiles estratégicos:** existe un piloto de 38 campeones con procedencia `Inferred` y confianza
  0.35 para validar el motor. No es un dato observado ni curado y la app lo etiqueta como pendiente de
  revisión experta. Los campeones sin perfil quedan desconocidos, no en cero.
- **Catálogo de objetos y runas:** fichero curado `crates/loldraft-engine/data/items-v1.json`,
  versionado y validado con Serde. Describe qué hace cada objeto o página, sus iconos y nombres
  oficiales, sus etiquetas de encaje y las amenazas que contrarresta. Incluye plantillas por familia y
  una asignación por campeón (con sobrescritura opcional de núcleo y runas); sin plantilla se aplican
  reglas por función. La recomendación se etiqueta como plantilla curada o derivada de reglas
  (`Inferred`), nunca como estadística observada; la revisión experta sigue pendiente.
  `scripts/audit-item-catalog.mjs` contrasta el catálogo con el parche en vivo: avisa de objetos
  legendarios sin catalogar y falla si un objeto o nombre ya no existe. No reescribe datos curados.
- **CounterStats:** se enlaza la página pública de cada rival leído; no se scrapea, copia ni
  redistribuye su contenido. La app solo abre el enlace en el navegador del sistema.
- **ProBuilds:** el panel de objetos enlaza la página pública del campeón para comparar con builds de
  profesionales. Su licencia prohíbe copiar o mostrar su contenido, así que no se scrapea ni se
  reutiliza: solo se abre el enlace externo.
- **Lolalytics:** no se scrapea. Su API privada no autoriza uso de terceros; la estrategia de
  estadísticas se decidió con la API oficial de Riot y datos importados.

## Documentación de diseño

| Documento | Contenido |
| --- | --- |
| [01 — Arquitectura](docs/01-arquitectura.md) | Crates, dependencias, capas, flujo de datos, decisiones técnicas |
| [02 — Modelo de datos](docs/02-modelo-datos.md) | Entidades, esquema SQLite, procedencia y versionado |
| [03 — Motor de composiciones](docs/03-motor-composiciones.md) | Arquetipos, agregación, necesidades, sinergias y matchups |
| [04 — Motor de recomendaciones](docs/04-motor-recomendaciones.md) | Features, puntuación, contexto, baneos y explicaciones |
| [05 — Fuentes de datos](docs/05-fuentes-datos.md) | Data Dragon, Leaguepedia, Lolalytics, caché y licencias |
| [06 — Interfaz](docs/06-interfaz.md) | Pantallas, layout del simulador, estados vacíos y accesibilidad |
| [07 — Plan por fases](docs/07-plan-fases.md) | Entregables y criterios de aceptación por fase |
| [08 — Riesgos y decisiones](docs/08-riesgos-decisiones.md) | Riesgos, mitigaciones y ADRs |
| [09 — Curación de perfiles](docs/09-curacion-perfiles.md) | CLI, flujo de revisión, límites y honestidad |
| [10 — Modo SoloQ](docs/10-modo-soloq.md) | Sincronización con el cliente, pick propio y mejor pick por turno |
| [11 — Instalación y actualizaciones](docs/11-instalacion-actualizaciones.md) | Instalador NSIS, canal `latest.json`, botón de actualizar y límites |

## Principios del proyecto

1. **El dominio no hace I/O.** El estado del draft, el análisis y la puntuación son funciones puras
   y deterministas, testeables sin base de datos ni red.
2. **Desconocido no es cero.** Si falta un dato estratégico, se muestra como desconocido y reduce la
   confianza; nunca se inventa un valor neutro que parezca observado.
3. **Procedencia explícita.** Todo dato estratégico indica fuente, confianza y versión: curado,
   importado, inferido o de Riot.
4. **Estadísticas con contexto.** Winrate con *shrinkage*, tamaño de muestra y separación entre solo
   queue y competición. Nunca es el único criterio.
5. **Primero funcional, luego complejo.** Sin misticismo: nada de ML, LLMs ni microservicios antes de
   que el simulador y el análisis básico funcionen y estén medidos.

## Aviso legal

LolDraft no está afiliado ni respaldado por Riot Games. League of Legends y todos los activos
asociados son propiedad de Riot Games, Inc. Los datos de terceros conservan su licencia y atribución
(ver [05 — Fuentes de datos](docs/05-fuentes-datos.md)).

## Licencia

El código se publica bajo licencia doble **MIT OR Apache-2.0**:

- [LICENSE-MIT](LICENSE-MIT)
- [LICENSE-APACHE](LICENSE-APACHE)

Puedes elegir la que prefieras. Los retratos, nombres y demás contenido de Riot Games **no** quedan
cubiertos por esa licencia y siguen siendo propiedad de Riot Games, Inc. La tipografía Cinzel
(`apps/desktop/public/fonts`) se distribuye bajo la **SIL Open Font License 1.1**, cuyo texto viaja
junto a los ficheros de la fuente.
