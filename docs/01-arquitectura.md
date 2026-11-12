# 01 — Arquitectura

> Estado: diseño aprobado para implementación por fases. No describe código existente.

## 1. Vista general

```text
┌──────────────────────────────────────────────────────────────────┐
│ apps/desktop  (React + TypeScript + Tailwind)                    │
│  Simulador · Análisis · Estadísticas · Configuración             │
└───────────────▲──────────────────────────────────────────────────┘
                │ comandos Tauri (JSON) + eventos de progreso
┌───────────────┴──────────────────────────────────────────────────┐
│ src-tauri  (loldraft-app)                                        │
│  Estado de aplicación · comandos · DTOs · mapeo de errores       │
└───────┬───────────────────────┬───────────────────────┬──────────┘
        │                       │                       │
┌───────▼────────┐   ┌──────────▼──────────┐   ┌────────▼─────────┐
│ loldraft-engine│   │ loldraft-data       │   │ loldraft-sources │
│ recomendación  │   │ SQLite · repos      │   │ HTTP · caché     │
│ explicaciones  │   │ migraciones · seeds │   │ importadores     │
└───────┬────────┘   └──────────┬──────────┘   └────────┬─────────┘
        │                       │                       │
        └───────────────────────┴───────────────────────┘
                                │
                       ┌────────▼─────────┐
                       │ loldraft-domain  │
                       │ draft · análisis │
                       │ tipos puros      │
                       └──────────────────┘
```

Regla de dependencias: **nada apunta hacia arriba**. `domain` no conoce SQLite, HTTP, Tauri ni
React. `engine` conoce el dominio, no la base de datos. `sources` produce lotes normalizados; no
escribe en SQLite. La app orquesta: descarga → valida → persiste → analiza → recomienda.

## 2. Estructura de carpetas

```text
LolDraft/
├─ Cargo.toml                     # workspace
├─ rust-toolchain.toml            # toolchain fijado (MSVC)
├─ crates/
│  ├─ loldraft-domain/
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ ids.rs                # ChampionKey, PatchId, Role, Side...
│  │     ├─ champion.rs           # perfil estratégico, rol, arquetipos
│  │     ├─ archetype.rs          # taxonomía y escala
│  │     ├─ draft/
│  │     │  ├─ format.rs          # formatos y secuencias de acciones
│  │     │  ├─ state.rs           # DraftState + validación
│  │     │  ├─ history.rs         # undo/redo por snapshots
│  │     │  └─ assignment.rs      # asignación óptima de roles
│  │     ├─ analysis/
│  │     │  ├─ composition.rs     # agregación de arquetipos y perfiles
│  │     │  ├─ needs.rs           # carencias y demandas del rival
│  │     │  ├─ synergy.rs         # sinergias curadas + derivadas
│  │     │  └─ matchup.rs         # matchup de línea y de equipo
│  │     └─ error.rs
│  ├─ loldraft-engine/
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ candidate.rs          # universo de candidatos legales
│  │     ├─ features.rs           # 8 features normalizadas
│  │     ├─ weights.rs            # perfiles por fase, validación
│  │     ├─ score.rs              # puntuación y ranking
│  │     ├─ explain.rs            # Reason/Contribution deterministas
│  │     ├─ bans.rs               # puntuación de baneos
│  │     └─ confidence.rs
│  ├─ loldraft-data/
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ db.rs                 # pool, PRAGMAs, transacciones
│  │     ├─ migrations/
│  │     ├─ repo/                 # champions, stats, drafts, settings...
│  │     ├─ seed.rs               # taxonomía y perfiles versionados
│  │     └─ import.rs             # staging → validación → promoción
│  ├─ loldraft-sources/
│  │  └─ src/
│  │     ├─ lib.rs                # trait DataSource + ImportBatch
│  │     ├─ http.rs               # timeouts, reintentos, User-Agent
│  │     ├─ cache.rs              # caché HTTP condicional
│  │     ├─ ddragon.rs
│  │     ├─ leaguepedia.rs
│  │     ├─ lolalytics.rs         # deshabilitado por defecto (ver doc 05)
│  │     └─ import/{csv,json}.rs
│  └─ loldraft-eval/              # offline: backtesting, no se empaqueta
├─ src-tauri/                     # paquete loldraft-app + tauri.conf.json
├─ apps/desktop/                  # React + Vite + Tailwind
│  └─ src/
│     ├─ app/                     # shell, router, providers
│     ├─ features/{draft,analysis,stats,settings,champions}/
│     ├─ components/
│     ├─ lib/{api.ts,generated-types.ts,i18n.ts}
│     └─ styles/
├─ data/
│  ├─ taxonomy/                   # arquetipos, traits, roles (JSON versionado)
│  ├─ profiles/                   # perfiles curados por parche
│  └─ fixtures/                   # datos sintéticos SOLO para tests
└─ docs/
```

## 3. Crates y responsabilidades

### `loldraft-domain`
Tipos, reglas y cálculos puros. Sin `reqwest`, `rusqlite`, `tauri`, `tokio`.
Dependencias: `serde`, `thiserror`.

Contiene:
- Estado del draft y máquina de turnos para cada formato.
- Asignación óptima de roles.
- Análisis de composición, necesidades, sinergias y matchups.
- Tipos de salida con confianza y procedencia.

### `loldraft-engine`
Puntuación multicriterio y explicaciones. Depende de `domain`.
No consulta la base de datos: recibe un `DraftSnapshot` ya resuelto (campeones, perfiles,
estadísticas y configuración). Esto permite tests con fixtures y backtesting reproducible.

### `loldraft-data`
Persistencia SQLite, migraciones, repositorios, seeds e importación transaccional.
Depende de `domain`. No conoce Tauri.
`rusqlite` (bundled SQLite) + `rusqlite_migration`. Las consultas son bloqueantes; la app las
ejecuta con `tokio::task::spawn_blocking` y un pool acotado.

### `loldraft-sources`
Adaptadores de red y parseo. Depende de `domain`. Recibe una configuración y devuelve
`ImportBatch` normalizado; nunca escribe en la base de datos.
`reqwest` (rustls), `tokio`, `serde`, caché propia en disco.

### `src-tauri` (`loldraft-app`)
Estado compartido (`AppState`), comandos, DTOs de transporte, validación de entrada y mapeo de
errores. Es la única capa que conoce Tauri.

### `loldraft-eval`
Herramienta offline (binario/`xtask`) para backtesting y ablaciones. No se incluye en el bundle de
la aplicación. Puede leer la misma base de datos y el mismo motor.

## 4. Flujo de una acción de draft

```text
UI: clic en campeón
  → invoke("draft_action", { sessionId, action })
  → validación de sesión y payload
  → DraftState::apply(action)            (dominio, puro)
  → asignación de roles y análisis       (dominio, puro)
  → engine::recommend(snapshot, weights) (motor, puro)
  → persistencia de la sesión            (data, spawn_blocking)
  → DraftView (estado + análisis + recomendaciones)
  → render en UI
```

La UI no calcula reglas ni puntuaciones. No hay dos fuentes de verdad: el estado autoritativo vive
en Rust y cada comando devuelve la vista completa. Con 20 acciones por draft el coste es
despreciable y se elimina la clase de bugs de estado duplicado.

## 5. Contratos Tauri

Comandos previstos (nombres estables desde Fase 1):

| Comando | Entrada | Salida |
| --- | --- | --- |
| `app_bootstrap` | — | parche actual, campeones, taxonomía, ajustes, estado de datos |
| `draft_new` | `formatId`, `side` | `DraftView` |
| `draft_action` | `sessionId`, `action` | `DraftView` |
| `draft_undo` / `draft_redo` | `sessionId` | `DraftView` |
| `draft_load` / `draft_save` / `draft_list` | — | sesiones persistidas |
| `draft_recommend` | `sessionId` | `RecommendationSet` |
| `draft_bans` | `sessionId` | `BanRecommendationSet` |
| `analysis_get` | `sessionId` | `MatchupAnalysis` |
| `champions_list` / `champion_detail` | filtros / `championKey` | `ChampionView` |
| `settings_get` / `settings_update` | — | `SettingsView` |
| `data_status` | — | datasets, última actualización, licencias |
| `data_sync` | `sourceId`, `options` | progreso por eventos + resultado |
| `data_import` | ruta + tipo | resumen de importación |
| `stats_query` | filtros | series y tablas |

DTOs:
- Se definen en `src-tauri/src/dto.rs`, separados de los tipos de dominio para no acoplar la API
  pública a cambios internos.
- Generación de tipos TypeScript con `ts-rs` (derive `TS`) y un test que falla si el archivo
  generado está desactualizado. Evita drift manual.
- Fechas como epoch milisegundos (enteros) en la frontera; la UI formatea.
- Errores: `{ code, message, details? }` con códigos estables (`DRAFT_INVALID_ACTION`,
  `DATA_SOURCE_RATE_LIMITED`, `DATASET_INCOMPATIBLE`...). Nunca se expone un `Debug` de Rust.

## 6. Estado y concurrencia

- `AppState` contiene el pool SQLite, la configuración cargada, el catálogo en memoria y un mapa
  de sesiones activas.
- Las sesiones de draft se serializan por `sessionId` con un `Mutex` pequeño; operaciones de DB
  fuera del lock.
- Una única conexión escritora lógica + pool de lectores. SQLite en modo WAL, `busy_timeout`,
  `foreign_keys=ON`, `synchronous=NORMAL`.
- Descargas y sincronizaciones son trabajos en segundo plano con progreso por eventos. La UI puede
  seguir simulando mientras se actualizan datos; los datasets nuevos se promocionan atómicamente.
- Sin `async` en el dominio. `tokio` solo en red y en el puente de comandos.

## 7. Errores y observabilidad

- `thiserror` para errores tipados por capa.
- En la frontera Tauri se mapea a `AppError` serializable con código estable y mensaje en español.
- `tracing` + `tracing-subscriber` con rotación de logs en el directorio de datos de la app.
- Los errores de red incluyen fuente, URL (sin secretos), estado HTTP e intentos.
- Los panics se capturan en comandos y se registran; no se usa `unwrap()` en rutas de producción.

## 8. Seguridad

- Capacidades Tauri mínimas: sin plugin de shell, sin fs remoto. Las imágenes se sirven por el
  protocolo de assets con allowlist del directorio de caché.
- CSP estricta; la UI no carga scripts ni iframes remotos.
- Toda la red se hace desde Rust. Nada de `fetch` a terceros desde el webview.
- Consultas SQL siempre parametrizadas. Migraciones empaquetadas, nunca descargadas.
- Claves de API (si se usa la API oficial de Riot) en el llavero del sistema (`keyring`), no en
  SQLite ni en el frontend.
- Importación de archivos: límite de tamaño, validación de esquema y transacción con rollback.

## 9. Rendimiento

Órdenes de magnitud esperados, no optimizaciones prematuras:

- 173 campeones × 20 arquetipos × 18 traits: análisis de una composición en microsegundos.
- Evaluación de candidatos: como máximo 173 candidatos × 120 asignaciones de rol (5!); en Rust son
  decenas de miles de operaciones, muy por debajo de 10 ms.
- Matriz de sinergias precalculada al cargar el catálogo (~15 000 pares), invalidada por versión de
  perfiles.
- La UI renderiza 173 tarjetas con imágenes lazy; no hace falta virtualizar, pero se medirá antes
  de añadir complejidad.

Se medirá con `criterion` en `loldraft-engine` y con un benchmark de arranque. Solo se optimiza lo
que aparezca en el perfil.

## 10. Build y distribución

- Tauri v2 con `tauri.conf.json`; `frontendDist = ../apps/desktop/dist`, `devUrl` de Vite.
- Workspace Cargo con `resolver = "2"`, edición 2021 o superior.
- Toolchain MSVC (`stable-x86_64-pc-windows-msvc`). El toolchain GNU detectado no se usará para
  Tauri.
- CI (GitHub Actions): `fmt`, `clippy -D warnings`, `cargo test`, `npm ci && npm run build` en
  Windows y Linux. El empaquetado de instaladores solo en tags.
- Datos: la app arranca con la taxonomía empaquetada y sincroniza campeones desde Data Dragon. Los
  perfiles estratégicos van versionados en `data/profiles`; las estadísticas se importan y viven en
  el directorio de datos del usuario.

## 11. Decisiones que se evitan deliberadamente

- **No** haber optado por una única crate monolítica.
- **No** poner la lógica del draft en el frontend.
- **No** usar un ORM pesado: el esquema es pequeño y las consultas son conocidas.
- **No** usar un LLM para puntuar ni para explicar. Las explicaciones se derivan de las
  contribuciones numéricas.
- **No** construir un DSL de reglas configurable antes de tener reglas tipadas y testeadas.
- **No** añadir caché, colas o workers distribuidos: un proceso de escritorio no los necesita.
