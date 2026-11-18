# 06 — Interfaz de usuario

> Estado: diseño. Implementación de la pantalla de simulación en Fase 1; análisis y recomendaciones
> se añaden en Fases 2 y 3.

## 1. Principios visuales

- **Temática esports, pero sobria:** fondo oscuro, paneles con contraste, azul/rojo como identidad
  de lado, dorado para énfasis. Nada de neones gratuitos ni animaciones que distraigan.
- **Jerarquía antes que densidad:** el estado del draft y el siguiente turno deben leerse en menos
  de un segundo. El detalle vive en paneles expandibles.
- **Números honestos:** las barras muestran intervalos cuando falta información; no se pinta una
  barra precisa si el dato es desconocido.
- **Una sola fuente de verdad:** la UI representa lo que devuelve Rust. No recalcula reglas,
  puntuaciones ni análisis.
- **Español primero, i18n preparada:** todos los textos en un diccionario; los nombres de campeón
  dependen del locale de Data Dragon.

## 2. Sistema de diseño

### 2.1 Tokens

| Token | Valor inicial | Uso |
| --- | --- | --- |
| `bg` | `#0B0F14` | Fondo de aplicación |
| `surface` | `#111823` | Paneles |
| `surface-2` | `#17202C` | Tarjetas, hover |
| `border` | `#243244` | Separadores |
| `text` | `#E6EDF5` | Texto principal |
| `muted` | `#93A4B8` | Texto secundario |
| `blue` | `#3B82F6` | Lado azul |
| `blue-soft` | `#1E3A5F` | Fondos de lado azul |
| `red` | `#EF4444` | Lado rojo |
| `red-soft` | `#4A1D1D` | Fondos de lado rojo |
| `gold` | `#D4AF37` | Recomendación principal |
| `success` | `#22C55E` | Fortaleza / dato confirmado |
| `warning` | `#F59E0B` | Dato parcial |
| `danger` | `#EF4444` | Error / carencia |

Tipografía: `Inter` o la fuente del sistema; números con `font-variant-numeric: tabular-nums`.
Espaciado en múltiplos de 4. Radios de 8–12 px. Sombras sutiles, sin glassmorphism excesivo.

### 2.2 Componentes base

- `SidePanel` (azul/rojo), `ChampionSlot`, `BanRow`, `RoleBadge`, `ScoreBadge`,
  `ConfidenceBadge`, `AnalysisBar` (con intervalo), `DataStatus`, `EmptyState`, `ErrorBanner`,
  `Drawer`, `Toast`.
- Los iconos de rol son SVG propios o un set con licencia clara; no se depende de imágenes
  remotas.
- Los retratos se sirven desde la caché local; mientras cargan, un placeholder neutro (nunca un
  retrato de otro campeón).

## 3. Arquitectura de pantallas

```text
Inicio
├─ Nuevo draft ──────────────► Simulador
├─ Continuar draft ──────────► Simulador (sesión guardada)
├─ Draft de SoloQ ───────────► Modo SoloQ (draft manual + mejor pick)
├─ Análisis de composición ──► Laboratorio de análisis
├─ Estadísticas ─────────────► Explorador de datos
├─ Configuración ────────────► Ajustes
└─ Datos / Acerca de ────────► Estado de fuentes y licencias
```

### 3.1 Inicio

- Estado del parche y de los datos ("Parche 16.22 · datos del 11/11 · perfiles v3").
- Acción principal: **Nuevo draft**.
- Sesiones recientes con formato, lado, progreso y fecha.
- Aviso si faltan datos o hay una actualización disponible.
- Sin tarjetas de estadísticas inventadas. Si no hay datos, se explica qué falta y cómo obtenerlos.

### 3.2 Simulador

Layout de escritorio (≥ 1280 px):

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Parche 16.22 · Formato estándar · Lado azul · Datos ✓ · [Deshacer] [Rehacer] │
├───────────────────────┬──────────────────────────┬───────────────────────────┤
│ AZUL                  │     SECUENCIA DEL DRAFT  │ ROJO                      │
│                       │                          │                           │
│ Picks                 │  B1 ban  ✔               │ Picks                     │
│  [slot] [slot] ...    │  R1 ban  ✔               │  [slot] [slot] ...        │
│  rol / campeón        │  R2 ban  ✔               │  rol / campeón            │
│                       │  B2 ban  ◀ turno         │                           │
│ Bans                  │  ...                     │ Bans                      │
│  [x] [x] [x] [x] [x]  │  20 acciones             │  [x] [x] [x] [x] [x]      │
│                       │                          │                           │
│ Identidad: engage     │  Pick actual: azul B2    │ Identidad: poke/siege     │
│ Confianza: 72 %       │  Quedan 14 acciones      │ Confianza: 64 %           │
├───────────────────────┴──────────────────────────┴───────────────────────────┤
│ Buscar [_________]  Rol [Todos ▾]  Arquetipo [Todos ▾]  Solo disponibles ☑  │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ... 173 campeones                        │
│ │img │ │img │ │img │ │img │ │img │                                          │
│ └────┘ └────┘ └────┘ └────┘ └────┘                                          │
├───────────────────────────────────────────────┬──────────────────────────────┤
│ RECOMENDACIONES                               │ ANÁLISIS                     │
│ 1. Campeón A  87  ↑ engage +0.31              │ Fortalezas / carencias       │
│    "Cubre engage y añade daño mágico"         │ Teamfight  ███████░░ 72      │
│ 2. Campeón B  81  …                           │ Engage    ████░░░░░ 41       │
│ ...                                           │ ...                          │
│ [Ver explicación completa]                    │ [Comparar equipos]           │
└───────────────────────────────────────────────┴──────────────────────────────┘
```

Comportamiento:

- **Zona superior:** parche, formato, lado, estado de datos, undo/redo. El parche es informativo;
  cambiarlo exige iniciar/editar la configuración del draft.
- **Paneles de equipo:** picks con retrato, nombre, rol asignado y confianza del rol. Bans en una
  fila compacta. El rol se puede cambiar con un desplegable (flex picks).
- **Secuencia:** 20 acciones agrupadas por fase; la actual se resalta; las pasadas se pueden
  inspeccionar; no se puede saltar a una acción futura.
- **Identidad de composición:** etiqueta principal ("engage", "poke/siege", "frontline DPS"...) con
  confianza. Si no hay identidad clara, se dice.
- **Selector:** búsqueda por nombre, filtro por rol, arquetipo, disponibilidad; orden alfabético por
  defecto y opción "según recomendación". Los campeones no disponibles se muestran atenuados con el
  motivo (baneado/picado) y no se pueden seleccionar.
- **Recomendaciones:** lista ordenada; la primera con énfasis dorado. Cada tarjeta muestra
  puntuación, rol, confianza y 2–3 motivos. Al expandir: contribuciones, sinergias, matchups,
  carencias y riesgos.
- **Análisis:** resumen visual de ambas composiciones con intervalos; pestañas "Fortalezas",
  "Carencias", "Condiciones de victoria", "Matchups". No se muestran números sin contexto.
- **Detalle de campeón:** drawer lateral al hacer clic en un retrato; no abandona el simulador.
  Incluye arquetipos, traits, roles, estadísticas (con `n`, parche y fuente) y sinergias.

#### 3.2.1 Modo SoloQ

Pantalla aparte del simulador competitivo, pensada para responder a una sola
pregunta: qué pick encaja mejor en el turno propio de una partida de ranked.

```text
Tu situación ........ lado · turno de pick (1-5) · rol opcional
Draft ............... 5 huecos aliados (el propio se marca aparte),
                      5 rivales y 10 baneos opcionales
Acción .............. "Ver mejor pick"
Resultado ........... tarjeta del mejor pick + panel de recomendaciones
Importar ............ "Importar del cliente" (solo lectura, experimental)
```

- Cada hueco abre un buscador de campeones filtrado a los disponibles; no se
  pueden repetir campeones.
- Al cambiar cualquier dato se descarta el resultado anterior: nunca se muestra
  una recomendación calculada sobre otro estado.
- Si el cliente no está abierto, el error se explica y la entrada manual sigue
  disponible.
- Detalle del contrato y límites en [`10-modo-soloq.md`](./10-modo-soloq.md).

### 3.3 Laboratorio de análisis

- Selección libre de dos composiciones (5 + 5) sin seguir el orden del draft.
- Útil para estudiar drafts históricos o composiciones hipotéticas.
- Mismos paneles de análisis y comparación.
- No muestra recomendaciones de draft si no hay una secuencia de acciones; puede mostrar
  "sustituciones sugeridas" como análisis, no como pick.

### 3.4 Estadísticas

- Filtros: parche, cola (solo queue/pro), tier, región, rol, campeón.
- Tabla ordenable y gráficos simples (barras/líneas). Sin mapas de calor decorativos.
- Cada cifra muestra `n`, fuente y fecha.
- Si no hay datos para un filtro: estado vacío con explicación, nunca una tabla de ceros.
- Comparación de campeones lado a lado.
- Exportar CSV de la vista actual.

### 3.5 Configuración

Secciones:

1. **Draft:** formato, lado por defecto, orden de acciones (solo lectura si el formato es fijo),
   comportamiento de undo/redo.
2. **Datos:** fuente activa, prioridad pro/solo queue, parche, actualización manual, fecha de última
   actualización, importar archivo, borrar datasets.
3. **Motor:** sliders de prioridad de dimensiones y fuente de estadísticas, con preset
   "Competitivo", "SoloQ", "Equilibrado"; botón "Restaurar".
4. **Apariencia:** tema (oscuro por defecto; claro opcional más adelante), densidad, idioma.
5. **Avanzado:** pesos internos solo en modo experto, con aviso y exportación de la configuración.

Cada cambio de pesos muestra una vista previa del efecto en el draft actual antes de aplicar.

### 3.6 Datos / Acerca de

- Tarjetas por fuente: nombre, licencia, última sincronización, estado, filas, errores.
- Atribuciones completas y aviso de no afiliación con Riot.
- Checksum del dataset activo.
- Botón "Exportar informe de datos".

## 4. Estados y feedback

| Situación | UI |
| --- | --- |
| Cargando catálogo | Skeletons, no campeones falsos |
| Sin datos de DDragon | Estado vacío con acción "Sincronizar" |
| Perfil estratégico incompleto | Badge "Perfil pendiente"; análisis con intervalo |
| Sin estadísticas del parche | "Sin datos para 16.22"; no se usan datos de otro parche sin avisar |
| Modo offline | Banner "Sin conexión; usando datos del {fecha}" |
| Error de fuente | Banner con código, reintento y enlace al log |
| Acción inválida | Toast breve; el estado no cambia |
| Confianza baja | Badge ámbar "Datos limitados" y explicación |
| Draft completo | Resumen y botón "Guardar análisis" |

Los errores nunca bloquean la navegación. Un fallo de red no impide seguir simulando con los datos
locales.

## 5. Interacción y atajos

- `Ctrl/Cmd + Z` deshacer; `Ctrl/Cmd + Shift + Z` o `Ctrl/Cmd + Y` rehacer.
- `/` enfoca la búsqueda de campeones.
- Un clic selecciona al campeón; `Enter` o el botón «Confirmar» aplica la acción y `Esc` cancela la
  selección. `Esc` también cierra el drawer.
- La navegación de la cuadrícula con flechas queda pendiente; hoy se usan Tab y la búsqueda.
- El turno actual se anuncia con `aria-live="polite"`.
- Los atajos no se activan mientras se escribe en un campo.

## 6. Accesibilidad

- Contraste mínimo 4.5:1 para texto; 3:1 para elementos grandes.
- El color de lado no es el único indicador: siempre hay etiqueta "Azul"/"Rojo" e icono.
- Foco visible en todos los controles; navegación completa por teclado.
- Los gráficos tienen equivalente textual (tabla o lista).
- `prefers-reduced-motion` desactiva transiciones.
- Los badges de confianza incluyen texto, no solo un color.
- Los retratos tienen `alt` con el nombre del campeón; los decorativos, `alt=""`.

## 7. Responsive

- **≥ 1280 px:** tres columnas (equipos a los lados; turno y selector en el centro), secuencia
  compacta de 20 celdas sobre la fila y paneles de recomendaciones/cobertura/datos en tres columnas.
- **1024–1279 px:** mismas tres columnas con anchos mínimos; los paneles inferiores pasan a dos
  columnas y el selector mantiene la altura visible.
- **< 1024 px:** una columna; la secuencia se desplaza horizontalmente y el selector queda debajo de
  los equipos.
- La ventana mínima es 1180 × 720. En esa medida el turno actual y el selector de campeones siguen
  visibles sin desplazamiento vertical; el análisis inferior requiere scroll.
- No se promete una experiencia móvil completa: es una app de escritorio.

## 8. Rendimiento percibido

- Imágenes lazy con `loading="lazy"` y decodificación asíncrona; caché local.
- La cuadrícula de 173 campeones se renderiza sin virtualización; si se mide un problema, se añade.
- Las recomendaciones se recalculan al cambiar el draft; el comando debe responder en < 100 ms
  percibidos. Si tarda más, se muestra un indicador y se cancela el cálculo anterior.
- Evitar re-renderizar toda la cuadrícula al cambiar una recomendación: memoización por campeón.
- El análisis se actualiza con transición corta; no se animan barras durante un cálculo pesado.

## 9. Contenido y microcopy

- "Puntuación de ajuste", no "probabilidad de victoria".
- "Datos limitados" en lugar de "poco fiable" (menos alarmista y más preciso).
- "Perfil pendiente" en lugar de "sin datos" cuando el campeón existe pero no está curado.
- "Cubre engage" / "Aporta engage" en lugar de "tiene engage alto".
- Los motivos siempre incluyen el "por qué": "Cubre la carencia de engage (0.30 → 0.61) y añade
  daño mágico a un equipo con 82 % físico".
- No se usan signos de exclamación ni lenguaje de apuestas.

## 10. Lo que la UI no hará

- No mostrar una barra de winrate como argumento principal.
- No rellenar huecos con valores medios o inventados.
- No ocultar la confianza ni la fuente.
- No permitir editar pesos sin mostrar su efecto.
- No mezclar datos de solo queue y profesionales sin etiqueta.
- No usar animaciones o sonidos que dificulten el análisis.
