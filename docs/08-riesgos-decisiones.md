# 08 — Riesgos, decisiones y preguntas abiertas

## 1. Riesgos principales

| # | Riesgo | Probabilidad | Impacto | Mitigación |
| --- | --- | --- | --- | --- |
| R1 | Lolalytics no autoriza el uso de sus datos | Alta | Alto | Abstracción de fuentes; importación manual; permiso escrito; no scraping por defecto; el motor funciona sin winrates |
| R2 | Curación de 173 perfiles estratégicos es costosa y subjetiva | Alta | Alto | Rúbrica, doble revisor, confianza, propuestas `Inferred` revisadas, cobertura progresiva; nunca rellenar con inventos |
| R3 | Muestras profesionales pequeñas por parche/matchup | Alta | Medio | Shrinkage, intervalos, confianza visible, no concluir con `n` bajo |
| R4 | Sobreajuste de pesos al backtest | Media | Alto | Holdout temporal, ablaciones, sensibilidad, pocos parámetros, documentar límites |
| R5 | El motor se percibe como "caja negra" | Media | Alto | Contribuciones sumables, explicaciones por código, desglose UI, tests de dominancia |
| R6 | Entorno Tauri incompleto (sin MSVC/Node) | Alta (actual) | Medio | Instalar Build Tools + Node antes de Fase 1; verificar `tauri doctor` |
| R7 | Cambios de esquema en fuentes (Cargo/DDragon) | Media | Medio | Parsers versionados, fixtures, validación, fallo ruidoso, caché |
| R8 | Mezcla silenciosa de parches/colas | Media | Alto | Dataset por parche/cola; fallback visible; etiquetas obligatorias |
| R9 | Complejidad de UI y scope creep | Media | Medio | Fases con criterios; MVP del simulador; fearless y estadísticas avanzadas después |
| R10 | Dependencias innecesarias | Media | Bajo | Revisión por fase; std/Serde/Tokio/rusqlite/reqwest; evitar ORM y state managers pesados |
| R11 | Corrupción de SQLite o migración fallida | Baja | Alto | WAL, backups antes de migrar, transacciones, test de recuperación |
| R12 | Problemas legales de assets/datos | Media | Alto | Atribuciones, aviso de no afiliación, licencias por dataset, revisión antes de publicar |
| R13 | El score se interpreta como probabilidad | Alta | Medio | Microcopy explícito, nombre "ajuste estratégico", no mostrar % de victoria |
| R14 | Asignación de roles ambigua en flex picks | Media | Medio | Optimización de asignación, override manual, badge de ambigüedad |
| R15 | El motor recomienda siempre el mismo meta | Media | Medio | Pesos bajos de `power`, necesidad/sinergia, diversificación, contexto de fase |

## 2. Decisiones de arquitectura (ADRs)

### ADR-001 — Workspace de crates con dominio puro

**Decisión:** separar `domain`, `engine`, `data`, `sources` y `app`; el dominio no hace I/O.

**Motivo:** tests rápidos, backtesting reproducible, posibilidad de sustituir UI o fuentes sin tocar
las reglas. Evita el monolito de un único `main.rs` o un `lib.rs` gigante.

**Consecuencia:** algo más de boilerplate inicial; se acepta a cambio de claridad.

### ADR-002 — Tauri v2 + React/TypeScript + Tailwind

**Decisión:** Tauri v2, UI React, Tailwind, Vite. La UI es delgada; el estado autoritativo vive en
Rust.

**Motivo:** requisito del proyecto, buen rendimiento, binario pequeño frente a Electron, ecosistema
maduro. Tauri v2 es la versión soportada.

**Consecuencia:** requiere toolchain MSVC, WebView2 y Node. El toolchain GNU detectado no se usará.

### ADR-003 — SQLite con `rusqlite` y migraciones propias

**Decisión:** `rusqlite` (bundled) + `rusqlite_migration`; sin ORM. Escrituras en
`spawn_blocking`, WAL, una conexión escritora.

**Motivo:** el esquema es pequeño y las consultas son explícitas. Un ORM añadiría dependencia y
fricción sin beneficio.

**Consecuencia:** SQL manual y tests de migración obligatorios.

### ADR-004 — Motor determinista, sin LLM

**Decisión:** puntuación y explicaciones se calculan con features y plantillas. No hay modelo
generativo en el camino crítico.

**Motivo:** auditabilidad, reproducibilidad, coste cero, tests. Un LLM podría verbalizar mejor, pero
también inventar razones que no corresponden a la puntuación.

**Consecuencia:** las explicaciones serán algo más rígidas; se mejora con i18n y buena redacción de
plantillas. Un LLM opcional futuro solo podría reformular un JSON de motivos ya calculado.

### ADR-005 — Desconocido no es cero; intervalos y confianza

**Decisión:** los perfiles ausentes son `None`, no `0.0`. El análisis calcula cotas y solo afirma
carencias cuando el intervalo lo permite.

**Motivo:** el error más grave sería decir "falta engage" por un dato no cargado. También evita
barras de progreso falsamente precisas.

**Consecuencia:** UI más compleja y datos incompletos visibles; es el comportamiento correcto.

### ADR-006 — Asignación óptima de roles

**Decisión:** resolver la asignación de roles por optimización (5! = 120), no de forma codiciosa.

**Motivo:** los flex picks y los roles secundarios son centrales en el draft competitivo. Una
asignación codiciosa puede bloquear la mejor composición.

**Consecuencia:** hay que definir `proficiency` y permitir override; la ambigüedad se muestra.

### ADR-007 — Fuentes abstractas; Lolalytics no se scrapea por defecto

**Decisión:** trait `DataSource`; Data Dragon y Leaguepedia son fuentes principales. Lolalytics solo
con permiso o importación manual.

**Motivo:** su API interna es privada y prohíbe el uso por terceros; un scraper sería frágil y
legalmente riesgoso.

**Consecuencia:** la Fase 4 puede no incluir estadísticas de Lolalytics si no hay acuerdo. El motor
no depende de ellas.

### ADR-008 — Estadísticas con shrinkage y separación de colas

**Decisión:** nunca usar winrate crudo; prior + tamaño de muestra. Soloq y pro son datasets
distintos, con mezcla configurable.

**Motivo:** 30 partidas pro no equivalen a 30 000 de solo queue; los parches cambian el rendimiento.

**Consecuencia:** se muestran `n` y confianza; algunos campeones tendrán datos insuficientes y se
dirá.

### ADR-009 — Procedencia y versionado de datos

**Decisión:** todo perfil y dataset tiene fuente, confianza, versión y checksum. Las
recomendaciones guardadas referencian esas versiones.

**Motivo:** auditar resultados, comparar parches y evitar mezclas invisibles.

**Consecuencia:** más metadatos y migraciones; se asume.

### ADR-010 — El score es ajuste estratégico, no probabilidad

**Decisión:** la salida es un índice 0–100 y un ranking; no se presenta como probabilidad de
victoria.

**Motivo:** sin calibración con resultados, un porcentaje sería una mentira estadística.

**Consecuencia:** si en Fase 5 se calibra, se hará como métrica separada y validada, no renombrando
el score actual.

### ADR-011 — Fearless y formatos alternativos, después del estándar

**Decisión:** Fase 1 implementa el formato estándar. El modelo de formato queda preparado para
secuencias configurables y fearless.

**Motivo:** evitar complejidad prematura; el estándar ya cubre el caso principal.

**Consecuencia:** la UI y el dominio no se cierran a futuros formatos, pero no se prometen hasta
Fase 3/4.

### ADR-012 — Curación asistida, no datos generados sin revisión

**Decisión:** se puede usar una herramienta offline para proponer perfiles a partir de datos
estructurados o texto de habilidades, pero cada propuesta se marca `Inferred` y requiere revisión
para subir de confianza.

**Motivo:** 173 × 20 celdas es demasiado para empezar a mano, pero presentar inferencias como
hechos destruiría la credibilidad del proyecto.

**Consecuencia:** habrá una fase de curación visible; la cobertura será progresiva.

## 3. Problemas técnicos identificados en la propuesta inicial

1. **"Lolalytics como fuente principal" no es viable sin permiso.** La propuesta original asumía
   una API que no existe públicamente. Se corrige con la abstracción y el modo manual.
2. **"Winrate + matchup" no basta para el análisis de composición.** Se necesita el perfil
   estratégico y las necesidades; el winrate queda como una feature menor.
3. **Los roles secundarios no se resuelven con un campo de texto.** Requieren asignación
   optimizada, especialmente con dos flex picks.
4. **La explicación no puede ser una frase generada aparte.** Debe derivarse de las contribuciones,
   o será inconsistente con el ranking.
5. **Los datos profesionales son escasos por parche.** Sin shrinkage y confianza, el motor
   sobreajustará a 10 partidas.
6. **La curación de 173 campeones es un proyecto en sí.** Hay que planificarla, no esconderla en
   "Fase 2".
7. **El entorno actual no puede compilar Tauri.** Falta linker MSVC y Node; se resuelve antes de
   escribir la primera línea de UI.
8. **El score 0–100 se malinterpreta como probabilidad.** Se corrige con nombres y microcopy.
9. **Los formatos de draft no son un array de turnos trivial.** Hay fases de baneo, lado, y en
   fearless los campeones usados se acumulan en la serie. Se modela como máquina de estados.
10. **La UI puede duplicar el estado y desincronizarse.** Se evita con Rust como fuente única y
    vistas completas por comando.

## 4. Preguntas abiertas

Estas decisiones necesitan confirmación del usuario antes o durante la Fase 1:

1. **Entorno:** ¿se instalan automáticamente Visual Studio Build Tools y Node.js 22 LTS, o los
   instala el usuario manualmente? La instalación de Build Tools ocupa varios GB.
2. **Datos de Lolalytics:** ¿se intenta permiso escrito, se empieza con importación manual, o se
   prioriza la API oficial de Riot? La recomendación es permiso + importación manual como respaldo.
3. **Curación de perfiles:** ¿se hace manual desde el inicio (más lento, máxima calidad), o se
   permite una herramienta de propuestas asistidas marcadas como inferidas y revisadas después?
4. **Alcance de la primera versión distribuible:** ¿solo Windows, o también Linux/macOS? Afecta al
   CI y al empaquetado, no al núcleo.
5. **Formato de draft inicial:** ¿solo el estándar de las grandes ligas, o también fearless desde
   el principio? La recomendación es estándar primero.

## 5. Criterios para no seguir adelante

Se debe parar y replantear si:

- no hay forma legítima de obtener estadísticas y el usuario considera que sin ellas el producto no
  tiene valor (el motor sí funciona, pero cambia la propuesta);
- la curación de perfiles no alcanza una cobertura mínima razonable (por ejemplo, 80 % de los
  campeones jugables) y no hay un plan para terminarla;
- el backtesting no supera baselines simples y no se identifica una causa corregible;
- el mantenimiento de las fuentes consume más que el desarrollo del motor.
