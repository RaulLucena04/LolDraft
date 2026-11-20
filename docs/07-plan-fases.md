# 07 — Plan de desarrollo por fases

> Estado: fases 0–3 cerradas en código, publicadas hasta v0.1.3. La Fase 2 queda pendiente de
> revisión experta de datos y la Fase 3 de estadísticas observadas. Cada fase termina con una versión
> funcional y verificable. No se avanza si los criterios de aceptación no se cumplen.

## Fase 0 — Diseño (cerrada)

Entregables:

- arquitectura, modelo de datos, metodología de análisis y recomendación;
- investigación de fuentes y licencias;
- diseño de interfaz;
- plan de fases y riesgos.

Criterio de salida: revisión y aprobación del usuario. No se escribe código de aplicación antes.

---

## Fase 1 — Base de la aplicación

> **Estado:** implementada en v0.1.0. Ver checklist al final de la fase.

**Objetivo:** un simulador de draft real, persistente y testeado, sin recomendaciones todavía.

### Tareas

1. **Entorno**
   - Instalar Visual Studio Build Tools (MSVC + Windows SDK).
   - Instalar Node.js 22 LTS.
   - Cambiar el toolchain de Rust a `stable-x86_64-pc-windows-msvc`.
   - Verificar `cargo`, `node`, `npm`, WebView2 y `tauri doctor`.
2. **Workspace**
   - `Cargo.toml` con crates `loldraft-domain`, `loldraft-engine` (vacío/mínimo),
     `loldraft-data`, `loldraft-sources`, `src-tauri`.
   - `rust-toolchain.toml`, `rustfmt.toml`, `clippy` config, `.gitignore`, licencia.
   - CI con `fmt`, `clippy`, `test`, build de UI.
3. **Dominio del draft**
   - `DraftFormat` estándar con las 20 acciones y ambos lados.
   - `DraftState` inmutable + `apply(action)` con validación.
   - Reglas: no duplicar campeón, no picar/bannear fuera de turno, lado correcto, acción correcta.
   - Undo/redo por snapshots (20 acciones, coste trivial).
   - `DraftSession` con historial, formato, parche y lado.
   - Formato preparado para fearless en el futuro, sin implementarlo aún.
4. **Asignación de roles**
   - `RoleFit` y asignación óptima por permutaciones.
   - Override manual del rol.
   - Detección de roles sin cubrir.
5. **Datos**
   - Migraciones SQLite iniciales (campeones, roles, sesiones, ajustes, datasets).
   - Sincronización Data Dragon: `versions.json` → `champion.json` → imágenes en caché.
   - Upsert idempotente; campeones nuevos con perfil desconocido.
   - Semilla de roles: top, jungle, mid, adc, support.
   - Persistencia de sesiones y ajustes.
6. **Backend Tauri**
   - `app_bootstrap`, `draft_new`, `draft_action`, `draft_undo`, `draft_redo`,
     `draft_save/load/list`, `champions_list`, `settings_get/update`.
   - DTOs y generación de tipos TypeScript.
   - Errores estables y logs.
7. **UI**
   - Shell oscuro, navegación, pantalla de inicio.
   - Simulador: paneles azul/rojo, timeline de 20 acciones, selector de campeones con filtros,
     bans, picks, undo/redo, indicador de turno.
   - Drawer de detalle de campeón (identidad y roles; sin estadísticas inventadas).
   - Estado vacío y errores de sincronización.
8. **Tests**
   - Secuencia completa de 20 acciones para azul y rojo.
   - Acciones inválidas, duplicados, undo/redo, persistencia.
   - Asignación de roles óptima vs fuerza bruta.
   - Migraciones desde cero; sincronización idempotente con fixture de DDragon.
   - Test de tipos generados.

### Criterios de aceptación

- Un draft estándar completo se puede simular desde ambos lados con las 20 acciones correctas.
- No se puede seleccionar un campeón ya usado ni actuar fuera de turno.
- Undo/redo funciona en cualquier punto y no corrompe el estado.
- Al cerrar y reabrir, la sesión se recupera.
- Se muestran los 173 campeones de Data Dragon con su imagen y roles.
- Un campeón sin perfil estratégico se muestra como "perfil pendiente", nunca con datos falsos.
- `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` y el build de la UI pasan.
- La acción de draft responde en menos de 100 ms percibidos en una máquina de desarrollo normal.

### Estado de implementación

- [x] Entorno MSVC, Node y toolchain documentados.
- [x] Workspace, formato, clippy y CI de GitHub Actions.
- [x] Formato estándar de 20 acciones, validación y undo/redo.
- [x] Asignación óptima de roles y override manual.
- [x] Migraciones SQLite, semilla, sesiones y ajustes.
- [x] Sincronización de identidades Data Dragon sin pisar roles.
- [x] Comandos Tauri, DTOs y errores estables.
- [x] UI de inicio y simulador con drawer de campeón.
- [x] Historial de deshacer reconstruido al reabrir una sesión guardada.
- [x] Tests de dominio, datos, fuentes y vista de draft; build de UI.
- [ ] Retratos de campeones nuevos descargados por sincronización (hoy se empaquetan con la semilla).
- [x] Medición de referencia del backend (dev, IPC): bootstrap 3,9 ms; cargar sesión 1,2 ms;
      acción 1,1 ms; deshacer 1,6 ms; rehacer 1,5 ms (medianas, 173 campeones).
      No es un benchmark de la build de release ni incluye el render de React.

### Fuera de alcance

- Recomendaciones, análisis estratégico, estadísticas, importación pro, fearless.

---

## Fase 2 — Análisis estratégico

> **Estado:** en curso. Implementados: taxonomía v1.0.0 (20 arquetipos, 18 traits y rúbrica de cinco
> anclas), esquema de perfiles con procedencia, confianza y validación en `loldraft-engine`,
> persistencia SQLite de conjuntos completos (roles, arquetipos, traits, activación única y carga
> atómica), agregación de composiciones con intervalos, daño, curva, CC, necesidades, amenazas y
> condiciones de victoria, un piloto de 38 campeones marcado como `Inferred` con revisión pendiente,
> su importación/activación desde la app, un panel de análisis para ambos equipos y sinergias y
> lecturas de línea/equipo derivadas de reglas, y una CLI de curación (`worksheet`, `edit`, `review`,
> `approve`, `diff`, `check`) cuyos conjuntos revisados se importan desde la app. Sigue pendiente la
> revisión experta real del piloto y de 20–30 composiciones, además de las sinergias curadas y los
> matchups observados que dependen de ella.

**Objetivo:** que el motor explique qué es una composición, qué le falta y cómo se enfrenta a otra.

### Tareas

1. **Taxonomía**
   - Seed versionado de 20 arquetipos, 18 traits, roles y rúbricas.
   - Escala de intensidad y ejemplos ancla.
   - Validación de shares de daño.
2. **Perfiles**
   - Herramienta de curación (pantalla interna o CLI) para editar perfiles con procedencia y
     confianza.
   - Piloto revisado de 30–40 campeones; después, expansión por lotes.
   - Perfiles propuestos por heurísticas o por análisis de kit se marcan `Inferred` y requieren
     revisión; nunca se presentan como observados.
   - Estrategia de cobertura: los campeones sin revisar quedan como desconocidos y la UI lo dice.
3. **Motor de composición**
   - Agregación saturante con intervalos de desconocimiento.
   - Perfil de daño, curva, rango, CC, movilidad, waveclear, objetivos.
   - Necesidades: suelos, reglas de amenaza y balance de daño.
   - Sinergias curadas y derivadas.
   - Matchups de línea y de equipo.
   - Condiciones de victoria.
   - Confianza y explicaciones por señal.
4. **UI de análisis**
   - Panel de fortalezas/carencias con intervalos.
   - Comparación de ambos equipos.
   - Condiciones de victoria e interacciones clave.
   - Drawer de campeón con perfil y sinergias.
5. **Tests y calibración**
   - Propiedades de monotonía, intervalos, rangos y ausencia de NaN.
   - Revisión experta de 20–30 composiciones.
   - Análisis de sensibilidad de los parámetros.
   - Golden tests con fixtures sintéticos.

### Criterios de aceptación

- Cualquier composición de 0 a 5 campeones produce un análisis coherente y marcado como parcial si
  falta información.
- Un dato desconocido no se convierte en cero ni en fortaleza; se muestra el intervalo.
- Añadir un campeón nunca reduce la cobertura de un arquetipo.
- Las carencias confirmadas y las posibles carencias se distinguen visualmente.
- El análisis de una composición completa tarda < 10 ms en Rust.
- La revisión experta no encuentra errores graves de clasificación en el piloto.
- Los perfiles tienen fuente, confianza y versión; se puede reproducir un análisis antiguo.

### Estado de implementación

- [x] Taxonomía v1.0.0 sincronizada en SQLite y validada.
- [x] Perfiles con procedencia, confianza, roles versionados y carga atómica.
- [x] Agregación de composiciones con intervalos, daño, curva, necesidades y condiciones de victoria.
- [x] Esquema de propuestas y piloto inferido de 38 campeones con roles, arquetipos y traits.
- [x] Comandos Tauri para importar, activar y analizar; DTOs y errores estables.
- [x] Panel de análisis con intervalos, daño, carencias, condiciones de victoria, sinergias
      derivadas, lectura de líneas y ventajas de equipo, con etiqueta de parcial.
- [x] Etiquetado explícito del piloto como `Inferred`, con revisión pendiente y sin activación
      automática: la importación requiere una acción del usuario.
- [x] Tests de propuestas, correspondencia con el catálogo, propiedades del motor y rendimiento.
- [ ] Revisión experta del piloto y de 20-30 composiciones.
- [x] Sinergias derivadas entre pares y lecturas de línea/equipo, con intervalos y confianza.
- [ ] Sinergias curadas y matchups observados (requieren datos revisados).
- [x] Herramienta de curación por fichero (`loldraft-curate`) e importación de conjuntos revisados
      desde la app: `from-proposals`, `worksheet`, `edit`, `diff`, `review`/`approve` y `check`;
      la expansión por lotes queda cubierta por `from-proposals` y las selecciones `--champion`.
- [x] Drawer de campeón con perfil (roles, arquetipos, traits, daño, cobertura y procedencia) y
      sinergias derivadas con los compañeros del lado.

### Fuera de alcance

- Recomendaciones de pick/ban, estadísticas externas, backtesting.

---

## Fase 3 — Motor de recomendaciones

> **Estado:** implementada y publicada (v0.1.2). Implementados: features `need`, `answer`, `synergy`,
> `counter`, `flex`, `balance` y `safety` con renormalización de pesos cuando falta una señal, pesos
> por fase, riesgos, penalización por confianza, diversificación por rol y arquetipo, baneos desde la
> perspectiva del rival, explicaciones con parámetros y la invariante `contribuciones + riesgos =
> puntuación`. La UI muestra picks y baneos, el desglose, los motivos, las carencias cubiertas y los
> datos que faltan. `power` sigue ausente porque no hay estadísticas observadas; tampoco hay
> reasignación completa de roles al evaluar (se respetan las asignaciones existentes) ni filtros de
> alternativas.

**Objetivo:** recomendar picks y baneos explicables, contextualizados y no dominados por winrate.

### Tareas

1. **Features**
   - `need`, `answer`, `synergy`, `counter`, `power`, `flex`, `balance`, `safety`.
   - Normalización, ausencia de datos y renormalización de pesos.
   - Riesgos y diversificación.
2. **Puntuación**
   - Pesos por fase y modificadores de contexto.
   - Asignación de rol dentro de la evaluación.
   - `ranking_score` con penalización por incertidumbre.
   - Baneos desde la perspectiva del rival.
3. **Explicaciones**
   - Contribuciones, riesgos y códigos de motivo.
   - Invariante de suma.
   - Plantillas i18n; nada de texto libre generado.
4. **UI**
   - Panel de recomendaciones, tarjetas, detalle expandible.
   - Desglose de puntuación y confianza.
   - Alternativas y filtros.
   - Panel de baneos cuando toca banear.
5. **Tests**
   - Los 12 tests del documento 04.
   - Fixtures sintéticos con un caso "winrate alto pero mala elección".
   - Determinismo y rendimiento.

### Criterios de aceptación

- Cada recomendación incluye puntuación, rol, confianza, motivos, carencias cubiertas, sinergias,
  matchups y riesgos.
- La suma de contribuciones coincide con la puntuación mostrada.
- Un campeón con winrate muy alto pero sin encaje no gana por defecto.
- Los pesos cambian por fase y el mismo estado produce explicaciones distintas en early/late.
- El top-5 está diversificado por rol cuando hay alternativas.
- Los baneos se explican desde el plan del rival, no solo por winrate.
- Sin perfiles suficientes, la UI dice "datos limitados" y no inventa una recomendación.
- El cálculo completo de recomendaciones para un draft tarda < 100 ms.

### Estado de implementación

- [x] Features y pesos por fase, con renormalización al faltar señales.
- [x] Riesgos, penalización por confianza y diversificación por rol/arquetipo.
- [x] Baneos puntuados desde la perspectiva del rival.
- [x] Explicaciones con parámetros e invariante de suma, con tests.
- [x] Determinismo, dominancia, sensibilidad a pesos y rendimiento (<100 ms) testeados.
- [x] UI de picks y baneos con desglose, motivos, carencias cubiertas y datos que faltan.
- [x] `power` con estadísticas observadas: percentil del winrate ajustado por rol, procedencia
  expuesta y separación estricta soloq/pro.
- [ ] Reasignación completa de roles al evaluar.
- [ ] Filtros de alternativas.

### Fuera de alcance

- Datos externos, backtesting serio, optimización fina.

---

## Fase 4 — Integración de datos

> **Estado:** en curso. Implementados el esquema validado de estadísticas observadas, la importación
> CSV/JSON, el shrinkage, la persistencia en SQLite, la pantalla de importación/activación y la señal
> `power` conectada al motor con separación estricta soloq/pro y procedencia visible. También está
> implementado un dataset importable de **builds y runas** por campeón/rol/cola, con selección por
> winrate ajustado, persistencia y panel junto al pick. Pendientes: el cliente de Leaguepedia, la
> caché HTTP condicional y los drafts profesionales.

**Objetivo:** actualizar datos sin reinstalar y sin mezclar fuentes/licencias.

### Tareas

1. **Leaguepedia**
   - Cliente con rate limit, backoff, caché y User-Agent.
   - Parser versionado de tablas Cargo; mapeo de campeones/equipos/parches.
   - Importación incremental e idempotente.
   - Cuarentena de filas inválidas y resumen de importación.
2. **Estadísticas y builds**
   - Importador CSV/JSON genérico con esquema validado.
   - Dataset de builds y runas por campeón, rol y cola, con selección por winrate ajustado.
   - Adaptador de Lolalytics **solo si hay permiso**; en su defecto, importación manual.
   - Separación estricta soloq/pro, tier, región y parche.
   - Shrinkage y confianza en la capa de datos.
3. **Caché y datasets**
   - Caché HTTP condicional y de imágenes.
   - `dataset_id`, checksum, licencia, staging y promoción atómica.
   - Puntero de dataset activo por tipo/parche.
4. **UI de datos**
   - Estado de fuentes, última actualización, filas, errores.
   - Importar archivo, reintentar, borrar datasets antiguos.
   - Atribuciones y licencias.
   - Selector de parche y de fuente pro/soloq.
5. **Seguridad y robustez**
   - Límites de tamaño, validación, timeouts, reintentos.
   - Logs sin secretos.
   - Modo offline.

### Criterios de aceptación

- Se puede actualizar el catálogo y las estadísticas desde la app, sin reinstalar.
- Reimportar el mismo torneo no duplica datos.
- Un fallo de importación no corrompe el dataset activo.
- La app funciona sin conexión con el último dataset válido.
- Cada dato muestra fuente, parche, `n` y licencia.
- No se incluye ningún scraper de Lolalytics sin permiso documentado.
- Los drafts profesionales importados permiten consultar pick/ban, lado, equipo, torneo, parche y
  resultado.
- El motor puede ejecutarse con datos pro, soloq o una mezcla configurada.

### Fuera de alcance

- Entrenamiento de modelos, estadísticas en tiempo real, integración con la API de Riot en vivo.

---

## Fase 5 — Evaluación y optimización

> **Estado:** sin empezar. Requiere datos observados de la Fase 4 y drafts profesionales.

**Objetivo:** medir si el motor aporta valor y corregir sin sobreajustar.

### Tareas

1. **Backtesting**
   - Predicción del siguiente pick/ban con `k` acciones conocidas.
   - Métricas top-1, top-3, MRR y comparación con baselines (uniforme, presencia, winrate).
   - Holdout temporal por torneo; nunca aleatorio dentro del mismo torneo.
   - Ablaciones por feature y análisis de sensibilidad.
2. **Calibración**
   - Ajuste conservador de pesos y umbrales.
   - Validación en un conjunto no usado para ajustar.
   - Informe de resultados y limitaciones.
3. **Producto**
   - Pulido de UI, accesibilidad, atajos, estados vacíos.
   - Rendimiento y tamaño del instalador.
   - Empaquetado firmado, actualizaciones y migraciones.
   - Documentación de usuario y de datos.
4. **Cierre**
   - Revisión de licencias y atribuciones.
   - Test de recuperación ante corrupción de SQLite.
   - Plan de mantenimiento por parche.

### Criterios de aceptación

- Existe un informe reproducible con métricas y baselines.
- El motor supera al baseline de winrate/presencia en top-3 en el holdout, o se documenta que no.
- Las ablaciones muestran que ninguna feature domina de forma desproporcionada.
- No hay regresiones en los tests de fases anteriores.
- El instalador funciona en una máquina limpia de Windows 11.
- La app arranca y simula un draft sin conexión.

---

## Trabajo transversal

| Área | Cuándo | Criterio |
| --- | --- | --- |
| CI | Fase 1 | fmt, clippy, tests, build de UI |
| Versionado de datos | Fase 2 | perfiles y datasets reproducibles |
| Accesibilidad | Fase 1 en adelante | teclado, contraste, foco |
| i18n | Fase 1 | español, estructura para más idiomas |
| Seguridad | Fase 1/4 | capacidades mínimas, sin secretos en UI |
| Documentación | Cada fase | docs actualizados con el código |
| Licencias | Fase 4/5 | atribuciones y revisión legal |

## Estimación orientativa

Una persona, tiempo parcial:

| Fase | Estimación |
| --- | --- |
| 1 | 2–3 semanas |
| 2 | 3–5 semanas (la curación de perfiles es el mayor coste) |
| 3 | 2–4 semanas |
| 4 | 2–4 semanas (depende de permisos y calidad de fuentes) |
| 5 | 2–4 semanas |

No son compromisos: la curación estratégica y los permisos de datos pueden dominar el calendario.
Es preferible retrasar una fase que rellenar huecos con datos inventados.

## Definición de "hecho"

Una tarea está hecha cuando:

1. el código compila sin warnings en CI;
2. tiene tests que cubren el caso feliz y los errores;
3. la UI muestra estados de carga, vacío y error;
4. los datos tienen procedencia y confianza;
5. la documentación refleja el comportamiento real;
6. se ha probado con un draft completo, no solo con un caso aislado.
