# LolDraft — Draft Assistant competitivo

Aplicación de escritorio para simular, analizar y asistir drafts competitivos de League of Legends.
Núcleo en Rust, escritorio con Tauri v2 y UI en React + TypeScript + Tailwind CSS.

> **Estado del repositorio:** Fase 1 implementada; Fase 2 en curso. El simulador estándar de 20
> acciones funciona, persiste en SQLite y muestra los 173 campeones de Data Dragon con sus roles
> importados. La taxonomía v1, el esquema de perfiles, el motor de análisis y un piloto inferido de
> 38 campeones ya están integrados. El piloto se etiqueta siempre como `Inferred` y queda pendiente
> de revisión experta; las recomendaciones de pick/ban todavía no existen.

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

## Qué funciona ahora (Fase 1)

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
- **Datos honestos:** los perfiles estratégicos ausentes se muestran como pendientes; las celdas
  inferidas se etiquetan como `Inferred` y no se presentan como observadas. No se muestran ceros ni
  winrates falsos.

## Fase 2 en curso

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
- **Pendiente:** revisión experta del piloto y de 20–30 composiciones, sinergias curadas, matchups
  observados, herramienta de curación y perfiles revisados.

Todavía **no** incluye: recomendaciones de pick/ban, sinergias curadas ni matchups observados,
estadísticas externas, importación de drafts profesionales ni fearless draft.

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
| `crates/loldraft-engine` | Taxonomía, perfiles, propuestas, análisis de composiciones y futuras recomendaciones. Sin I/O. |
| `crates/loldraft-data` | SQLite, migraciones, semillas y CRUD de sesiones/ajustes. |
| `crates/loldraft-sources` | Clientes de fuentes externas; ahora Data Dragon. |
| `src-tauri` | Estado de aplicación, DTOs, comandos Tauri y errores estables. |
| `apps/desktop` | UI React + TypeScript + Tailwind. |
| `crates/loldraft-data/seeds/champions.json` | Semilla reproducible de identidad y roles. |
| `apps/desktop/public/champions` | Retratos empaquetados de Data Dragon. |
| `scripts/fetch-static-data.mjs` | Generador reproducible de la semilla y los retratos. |

### Comandos Tauri

`app_bootstrap`, `champions_list`, `data_status`, `data_sync_ddragon`, `draft_new`, `draft_action`,
`draft_undo`, `draft_redo`, `draft_set_role`, `draft_list`, `draft_load`, `draft_delete`,
`profiles_list`, `profiles_import_pilot`, `profiles_import_proposal`, `profiles_activate`,
`analysis_current`, `settings_get`, `settings_update`.

La UI no recalcula reglas ni asignaciones: consume los DTOs que produce Rust.

## Datos y procedencia

- **Identidad y retratos:** [Data Dragon](https://developer.riotgames.com/docs/lol#data-dragon) de
  Riot Games, parche 16.22.1. Los retratos están empaquetados para funcionar sin conexión.
- **Roles:** Meraki Analytics (`positions`), importados con `source: "meraki"` y confianza 0.8.
  La atribución se muestra en la UI. Los campeones sin dato quedan pendientes.
- **Perfiles estratégicos:** existe un piloto de 38 campeones con procedencia `Inferred` y confianza
  0.35 para validar el motor. No es un dato observado ni curado y la app lo etiqueta como pendiente de
  revisión experta. Los campeones sin perfil quedan desconocidos, no en cero.
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
