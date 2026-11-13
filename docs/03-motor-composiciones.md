# 03 — Motor de análisis de composiciones

> Estado: implementación parcial en `loldraft-engine` (v0.5.0-phase2-interactions). Ya funcionan la
> agregación saturante con intervalos, daño, curva, rango, CC, movilidad, waveclear, objetivos,
> suelos, balance de daño, reglas de amenaza, condiciones de victoria, confianza, señales
> explicables, sinergias derivadas y lecturas de línea/equipo. Las sinergias curadas y los matchups
> observados siguen pendientes. Los valores numéricos son parámetros iniciales documentados, no
> verdades universales ni datos de campeones reales.
>
> Desviaciones conscientes de la v0.5: (1) las reglas de amenaza escalan la necesidad por la
> cobertura actual de la herramienta en lugar de sumar una cantidad fija, para no pedir algo que ya
> está cubierto; (2) el balance de daño solo se afirma con ≥ 60 % del peso de oro conocido; (3)
> `range`, `mobility`, `lane_safety`, `gold_dependency` y `execution_difficulty` usan media ponderada,
> mientras las herramientas de equipo usan suma saturante; (4) no hay estadísticas externas todavía,
> así que la confianza no incluye `stat_confidence`; (5) sinergias y matchups se derivan de perfiles
> y reglas, nunca de resultados observados.

## 1. Objetivo y límites

El motor debe responder, para cualquier conjunto de campeones:

1. ¿Qué aporta esta composición y en qué grado?
2. ¿Qué le falta para funcionar como equipo?
3. ¿Qué amenazas plantea el rival y qué respuestas exige?
4. ¿Qué sinergias y anti-sinergias existen?
5. ¿Cómo se enfrentan ambas composiciones?
6. ¿Cuál es su condición de victoria más probable?

**No** debe responder "probabilidad de ganar". El resultado es un perfil estratégico y un conjunto
de señales explicables. La probabilidad requiere un modelo calibrado con resultados históricos y
queda fuera del alcance hasta la Fase 5, si se valida.

## 2. Entrada

```rust
pub struct TeamSnapshot {
    pub side: Side,
    pub picks: Vec<PickedChampion>, // 0..5, con rol asignado
    pub bans: Vec<ChampionKey>,
}

pub struct PickedChampion {
    pub key: ChampionKey,
    pub role: Role,
    pub assignment_confidence: f32,
}

pub struct AnalysisInput<'a> {
    pub team: &'a TeamSnapshot,
    pub enemy: &'a TeamSnapshot,
    pub profiles: &'a ProfileSet,
    pub stats: &'a StatsSnapshot,
    pub params: &'a AnalysisParams,
}
```

Los campeones sin perfil estratégico no se descartan: se analizan con intervalos de desconocimiento
(sección 4).

## 3. Asignación de roles

Un campeón puede tener varios roles viables. Asignar el rol de forma codiciosa ("el primero que
encaje") produce errores con flex picks y con dos campeones que comparten rol principal.

Con como máximo 5 campeones y 5 roles, se resuelve el **problema de asignación** por fuerza bruta
(120 permutaciones) maximizando:

```text
score(assignment) = Σ_i proficiency(i, role(i))
                  + λ_flex * Σ_i flex_bonus(i, role(i))
                  + λ_coherence * coherence(team, assignment)
```

- `proficiency` es la calidad del campeón en ese rol (0..1).
- `flex_bonus` premia asignaciones que dejan el rol menos obvio para el rival; en el análisis puro
  es pequeño, porque la composición debe evaluarse por lo que puede hacer, no por engañar.
- `coherence` evita, por ejemplo, dos campeones que solo funcionan como carry de recursos en el
  mismo reparto. En Fase 2 se limita a penalizar un exceso de `gold_dependency` conjunta.

La UI permite **sobrescribir** la asignación. El estado guarda el rol elegido y su confianza. Si el
usuario cambia un rol, se recalcula todo el análisis.

Para equipos incompletos, los roles libres se rellenan con `None` y el análisis se marca como
parcial. La confianza baja de forma explícita.

## 4. Desconocimiento: intervalos, no ceros

Para cada arquetipo `d` y campeón `i`:

- `a(i,d) ∈ [0,1]` si se conoce.
- `a(i,d) = ?` si no se conoce.
- `f(i,r) ∈ [0,1]` es el ajuste al rol asignado.
- Contribución efectiva: `e(i,d) = a(i,d) * f(i,r(i))`.

La agregación se calcula dos veces:

- **Cota inferior (`A_low`)**: los desconocidos valen `0`.
- **Cota superior (`A_high`)**: los desconocidos valen `1`.

Con la fórmula de la sección 5. El resultado es un intervalo `[A_low, A_high]`.

Reglas de diagnóstico:

| Condición | Lectura |
| --- | --- |
| `A_low ≥ threshold` | Fortaleza confirmada |
| `A_high < threshold` | Carencia confirmada |
| Intervalo cruza el umbral | `Unknown` / datos insuficientes |

Esto evita el error más peligroso de este tipo de herramientas: afirmar "falta engage" porque un
campeón no tiene el dato cargado. Además:

- `knowledge(d) = conocidos / picks`.
- `confidence(d) = media(confidence_i) * knowledge(d)`.
- Una carencia solo se muestra como tal si es confirmada; si es incierta, se muestra como
  "posible carencia" con su intervalo.

## 5. Agregación de arquetipos

La cobertura de un arquetipo no es la suma de cinco campeones: el primero define la identidad, el
segundo añade profundidad y el resto aporta marginalmente. Se usa una suma saturante de las
contribuciones ordenadas:

```text
e(1) ≥ e(2) ≥ ... ≥ e(5)
A_d = clamp( e(1) + 0.30 * e(2) + 0.10 * e(3), 0, 1 )
```

Propiedades:

- Un especialista definitorio (`e=1.0`) cubre `1.0`.
- Un especialista sólido (`e=0.7`) cubre `0.7`.
- Dos sólidos (`0.5, 0.5`) cubren `0.65`: hay profundidad, no duplicación infinita.
- Cinco incidentales (`0.25 × 5`) cubren `0.325`: el equipo no tiene una herramienta fiable.
- El resultado es monótono: añadir un campeón nunca reduce la cobertura de un arquetipo.

Los pesos `[1.0, 0.30, 0.10]` son parámetros de `AnalysisParams`, con tests de monotonía y
sensibilidad. No se presentan como una ley del juego.

### 5.1 Traits agregados

Para traits escalares (`mobility`, `durability`, `hard_cc`, etc.) se usa la misma suma saturante,
salvo que se indique lo contrario. Para shares de daño se usa una media ponderada por rol (sección
6), porque el daño es un reparto, no una cobertura.

### 5.2 Perfil de daño

El daño potencial no depende solo del kit, sino de los recursos que recibe cada rol. Se usa un peso
de oro por rol como aproximación:

| Rol | `gold_weight` inicial |
| --- | --- |
| Top | 0.95 |
| Jungle | 0.85 |
| Mid | 1.00 |
| ADC | 1.10 |
| Support | 0.45 |

```text
P_type = Σ_i gold_weight(role_i) * damage_share(i, type)
         ─────────────────────────────────────────────
                    Σ_i gold_weight(role_i)
```

Se obtienen `P_physical`, `P_magic`, `P_true`, normalizados. Se muestran como **reparto de amenaza
de daño**, no como daño real de una partida.

Indicadores derivados:

- `magic_need`: `P_magic < 0.30` → el equipo necesita daño mágico.
- `physical_need`: `P_physical < 0.30` → necesita daño físico.
- `damage_redundancy`: `max(P_physical, P_magic) > 0.70` → penalización por añadir más del mismo
  tipo.
- `true_damage` se informa aparte y no cuenta para el balance físico/mágico.

### 5.3 Curva de poder

Con los traits `early_power`, `mid_power`, `late_power`:

```text
E = Σ_i gold_weight(role_i) * early_power(i) / Σ_i gold_weight(role_i)
M = ...
L = ...
```

No se comparan valores absolutos entre parches. Se comparan **diferencias dentro del mismo draft y
parche** y se acompañan de la incertidumbre.

### 5.4 Perfil de rango, CC y movilidad

- `ranged_share`: media ponderada del trait `range` (0 = melee, 1 = rango largo).
- `melee_count`: número de picks con `range < 0.35`.
- `hard_cc` / `soft_cc`: suma saturante; `reliable_cc` pondera CC instantáneo o dificil de esquivar.
- `mobility`: media ponderada; se usa para vulnerabilidad a engage/catch, no como fortaleza aislada.
- `waveclear`, `objective_dps`, `siege`: suma saturante, porque son capacidades de equipo.

### 5.5 Confianza del análisis

```text
knowledge = celdas conocidas / celdas esperadas
assignment_confidence = confianza media de los roles asignados
overall = min(0.70*knowledge + 0.30*assignment_confidence, knowledge)
```

La confianza nunca supera `knowledge`: una asignación de roles segura no compensa perfiles
desconocidos. Si `knowledge < 0.6` o hay menos de cinco picks, la UI muestra «parcial» y el motor de
recomendaciones reducirá el peso de las features que dependen de perfiles, en lugar de fingir
precisión.

## 6. Necesidades de la composición

Una necesidad no es "todo lo que sería bueno". Es una carencia concreta frente a un umbral o una
amenaza del rival. Se calcula en tres capas.

### 6.1 Suelo de composición

Para un conjunto reducido de dimensiones, existe un mínimo funcional inicial:

| Dimensión | `floor` inicial | Razón |
| --- | --- | --- |
| `frontline` | 0.50 | Alguien debe ocupar espacio |
| `engage` | 0.35 | Iniciar o forzar peleas |
| `disengage` | 0.30 | No perder ante la primera init |
| `peel` | 0.30 | Proteger carries |
| `dps` | 0.50 | Daño sostenido en peleas |
| `burst` | 0.30 | Amenaza de eliminación |
| `anti_tank` | 0.25 | No morir ante frontline |
| `sustain` | 0.20 | Mantener presión y objetivos |
| `waveclear` (trait) | 0.35 | Defender torres y gestionar oleadas |
| `hard_cc` (trait) | 0.30 | Control fiable |

```text
base_need_d = max(0, floor_d - A_d)
```

Estos suelos son un punto de partida, no una regla competitiva universal. Se validan con
composiciones de referencia y se pueden desactivar por configuración. El motor nunca oculta que son
parámetros.

### 6.2 Demandas del rival

Tabla de reglas tipadas, cada una con `id`, condición, necesidades que añade, peso y racional:

| Regla | Condición (ejemplo inicial) | Necesidad añadida | Por qué |
| --- | --- | --- | --- |
| `ENEMY_ENGAGE` | enemigo `engage ≥ 0.60` y propio `disengage < 0.40` | `disengage +0.55`, `peel +0.30` | Sin reset, la init enemiga decide la pelea |
| `ENEMY_ENGAGE_FRONTLINE` | enemigo `engage ≥ 0.60` y propio `frontline < 0.45` | `frontline +0.45` | Hace falta un cuerpo que absorba la entrada |
| `ENEMY_POKE` | enemigo `poke ≥ 0.55`, propio `engage < 0.45`, `sustain < 0.35` | `engage +0.50`, `sustain +0.30` | El poke se castiga forzando la pelea o sosteniendo |
| `ENEMY_SIEGE` | enemigo `siege ≥ 0.50` y propio `waveclear < 0.40` | `waveclear +0.40`, `engage +0.25` | Sin limpiar oleadas, se pierde por asedio |
| `ENEMY_FRONTLINE` | enemigo `frontline ≥ 0.60` y propio `dps < 0.55` | `dps +0.40`, `anti_tank +0.45` | El frontline enemigo exige daño sostenido y anti-tanque |
| `ENEMY_ASSASSIN` | enemigo `assassin ≥ 0.60` o `burst ≥ 0.60`, propio `peel < 0.40` | `peel +0.45`, `frontline +0.25` | El carry debe sobrevivir a la primera ventana |
| `ENEMY_CATCH` | enemigo `catch ≥ 0.55` y propio `mobility < 0.40` | `mobility +0.35`, `utility +0.25` | Ser atrapado antes de la pelea es una derrota |
| `ENEMY_SPLITPUSH` | enemigo `splitpush ≥ 0.55` y propio `catch < 0.40` | `catch +0.40`, `waveclear +0.30` | Hay que castigar o contener la lateral |
| `ENEMY_WOMBO` | enemigo `wombo_combo ≥ 0.55` y propio `disengage < 0.45` | `disengage +0.50` | Una combo de área sin reset gana la pelea |
| `ENEMY_EARLY` | enemigo `early_game ≥ 0.60` y propio `early_power < 0.45` | `early_game +0.35`, `lane_safety +0.30` | Evitar que el early dicte el mapa |
| `ENEMY_SCALING` | enemigo `scaling ≥ 0.65` y propio `early_game < 0.45` | `early_game +0.35` | Si ambos escalan, el rival llega mejor |

Reglas de implementación:

- Cada regla es una función tipada con tests, no un DSL genérico en Fase 2. Si el número de reglas
  crece y se necesita edición sin recompilar, se migrará a una tabla de datos versionada.
- Las necesidades se combinan con `max`, no con suma, para evitar que una sola amenaza dispare diez
  carencias artificiales.
- Cada necesidad conserva `source: Baseline | EnemyThreat(rule_id) | DamageBalance` para poder
  explicarla.

### 6.3 Balance de daño

```text
if P_magic < 0.30: need_magic = (0.30 - P_magic) / 0.30 * 0.60
if P_physical < 0.30: need_physical = (0.30 - P_physical) / 0.30 * 0.60
```

Se expresa como necesidad de "fuente de daño del tipo contrario" y se evalúa por el incremento
marginal que aporta el candidato, no por su share aislado.

### 6.4 Vector final

```text
need_d = priority_d * clamp(max(base_need_d, threat_need_d), 0, 1)
```

`priority_d` permite que el usuario o un perfil competitivo dé más importancia a frontline/engage
que a siege. Por defecto todos valen 1.0; la UI de configuración puede bajarlos.

## 7. Sinergias

### 7.1 Estado implementado

La v0.5 implementa **solo sinergias derivadas**, es decir, reglas tipadas sobre arquetipos y traits
del mismo equipo. No hay todavía sinergias curadas par a par ni un agregado de equipo: inventar esos
pesos sin datos revisados produciría una puntuación con apariencia de precisión.

Cada regla exige una dimensión a cada campeón del par. La satisfacción de un requisito es
`clamp(valor × proficiency_del_rol / umbral, 0, 1)`. El resultado es la media de los dos requisitos,
con intervalo:

```text
score_low  = media de satisfacciones conocidas (desconocido = 0)
score_high = media de satisfacciones conocidas (desconocido = proficiency)
confianza  = media de confianza de las celdas conocidas × celdas_conocidas / 2
```

Reglas iniciales (umbral entre paréntesis):

| ID | Requisito izquierdo | Requisito derecho |
| --- | --- | --- |
| `engage_burst` | `engage` (0.50) | `burst` (0.50) |
| `engage_dps` | `engage` (0.50) | `dps` (0.50) |
| `wombo_aoe` | `wombo_combo` (0.50) | `aoe` (0.50) |
| `peel_dps` | `peel` (0.50) | `dps` (0.50) |
| `frontline_dps` | `frontline` (0.50) | `dps` (0.50) |
| `poke_siege` | `poke` (0.50) | `siege` (0.50) |
| `catch_burst` | `catch` (0.50) | `burst` (0.50) |
| `sustain_poke` | `poke` (0.50) | `sustain` (0.50) |
| `split_waveclear` | `splitpush` (0.50) | `waveclear` (0.50) |
| `objective_frontline` | `objective_dps` (0.50) | `frontline` (0.50) |

Reglas de presentación:

- una sinergia exige al menos una celda conocida y `score_high >= 0.40`; sin evidencia no se emite;
- cada regla aporta como máximo una pareja por equipo (la de mayor puntuación), para que la lista no
  se llene con «engage + DPS» repetido;
- se muestran como mucho ocho parejas por equipo;
- el par se evalúa en ambos órdenes y se conserva la orientación con mayor puntuación;
- el detalle de campeón evalúa un candidato contra los picks de su lado con `candidate_synergies`,
  sin añadirlo al equipo ni inventar compañeros;
- la puntuación es un ajuste estratégico `0..1`, nunca una probabilidad ni un winrate.

### 7.2 Sinergias curadas (pendiente)

El diseño previsto mantiene aristas curadas con etiqueta, nota y fuente:

- `EngageFollowup`: engage fiable + daño de área inmediato.
- `WomboCombo`: dos o más áreas que se solapan bien.
- `PeelCarry`: protección + carry inmóvil.
- `FrontlineCarry`: frontline + DPS de alto impacto.
- `LaneDuo`: botlane o jungla-línea con interacción directa.
- `Objective`: aceleración de objetivos.
- `PickChain`: CC encadenado sobre un objetivo.
- `ProtectHypercarry`: recursos concentrados en un escalado.

Cuando existan, la sinergia final de un par combinará curada y derivada con un descuento explícito
para la parte inferida. Hasta entonces, la UI solo muestra reglas derivadas y lo dice.

### 7.3 Sinergia de equipo (pendiente)

El agregado `Σ w_i · w_j · s(i,j)` requiere pesos por rol revisados. No se implementa todavía para no
inventar la importancia relativa de cada pareja.

## 8. Matchups

### 8.1 Lectura de línea (implementada)

Solo se evalúan roles con un pick en **ambos** equipos. Para cada campeón se calcula una métrica con
media ponderada y el mismo criterio de desconocimiento que el resto del motor:

```text
presión = 0.40·early_power + 0.25·range + 0.20·lane_safety + 0.15·sustain
all-in  = 0.45·burst + 0.30·single_target + 0.25·hard_cc
ventaja = métrica_azul − métrica_roja     // intervalo: [low_azul − high_rojo, high_azul − low_rojo]
```

La ventaja se expresa en puntos `[-1,1]`. `favored` solo se rellena si el intervalo no cruza el cero
(margen 0.02). Es una lectura derivada de perfiles, **no** un historial de enfrentamientos: no hay
estadísticas externas todavía y la UI lo indica.

### 8.2 Matchup de equipo (implementada)

Cada regla combina una herramienta ofensiva del equipo con la vulnerabilidad del rival:

```text
vulnerabilidad = 1 − cobertura_de_la_respuesta
ventaja_low    = ataque_low × vulnerabilidad_low
ventaja_high   = ataque_high × vulnerabilidad_high
favored        = lado atacante solo si ventaja_low > 0.05
```

El producto evita dos errores: declarar ventaja cuando el atacante no tiene la herramienta, y
declararla cuando el rival sí tiene la respuesta. Reglas iniciales:

| ID | Herramienta atacante | Respuesta rival |
| --- | --- | --- |
| `poke_sustain` | `poke` | `sustain` |
| `anti_tank_frontline` | `anti_tank` | `frontline` |
| `catch_mobility` | `catch` | `mobility` |
| `split_waveclear` | `splitpush` | `waveclear` |
| `engage_disengage` | `engage` | `disengage` |
| `dps_tank` | `dps` | `tank` |
| `burst_peel` | `burst` | `peel` |
| `siege_waveclear` | `siege` | `waveclear` |
| `wombo_disengage` | `wombo_combo` | `disengage` |

Filtros: ambas coberturas deben tener al menos una celda conocida; se descartan las ventajas cuyo
límite superior no llega a 0.15; se muestran como mucho ocho, ordenadas por intervalo. No se suman
los cinco carriles ni se publica un índice global de victoria.

### 8.3 Matchups observados (pendiente)

Cuando existan datos de la API oficial de Riot con muestra suficiente, el diseño previsto aplica
*shrinkage* empírico:

```text
wr_adj = (wins + k * prior) / (games + k)
```

- `prior`: winrate general del campeón en ese parche/rol, o 0.5 si no existe.
- `k`: fuerza del prior; inicial 100 para solo queue y 8 para profesional.
- `confidence = f(games, k)`: crece con la muestra y se muestra explícitamente.

Nunca se usará un matchup con `games = 0`. Esos datos se etiquetarán como `Imported` y no se
mezclarán con las lecturas derivadas de esta versión.

## 9. Condiciones de victoria

Se calcula una puntuación para cada condición y se muestran las que superan un umbral, con la
evidencia que las sostiene:

| Condición | Señales iniciales |
| --- | --- |
| `FrontToBack` | frontline ≥ 0.55, dps ≥ 0.60, peel ≥ 0.40 |
| `Dive` | engage ≥ 0.60, burst ≥ 0.55, assassin ≥ 0.50, mobility ≥ 0.50 |
| `PokeSiege` | poke ≥ 0.60, siege ≥ 0.50, disengage ≥ 0.45 |
| `Pick` | catch ≥ 0.55, burst ≥ 0.50, utility ≥ 0.40 |
| `Splitpush` | splitpush ≥ 0.55, waveclear ≥ 0.50, mobility ≥ 0.45 |
| `Scaling` | scaling ≥ 0.60, late_power ≥ 0.60, waveclear ≥ 0.45 |
| `EarlySnowball` | early_game ≥ 0.60, early_power ≥ 0.60, hard_cc ≥ 0.40 |
| `Wombo` | wombo_combo ≥ 0.55, aoe ≥ 0.55, engage ≥ 0.50 |

Si ninguna supera el umbral, el diagnóstico es "sin condición de victoria clara" y la confianza
baja. Eso es información útil, no un fallo del motor.

## 10. Salida

```rust
pub struct CompositionAnalysis {
    pub side: Side,
    pub picks: usize,
    pub archetypes: BTreeMap<ArchetypeId, Coverage>, // valor + intervalo + confianza
    pub damage: DamageProfile,
    pub curve: PowerCurve,
    pub range: RangeProfile,
    pub cc: CcProfile,
    pub mobility: f32,
    pub waveclear: f32,
    pub objective_dps: f32,
    pub synergies: Vec<SynergySignal>,
    pub needs: Vec<Need>,
    pub win_conditions: Vec<WinCondition>,
    pub strengths: Vec<Signal>,
    pub weaknesses: Vec<Signal>,
    pub knowledge: f32,
    pub confidence: f32,
}

pub struct Coverage {
    pub value_low: f32,
    pub value_high: f32,
    pub value_estimate: f32, // solo si el intervalo es estrecho
    pub confidence: f32,
    pub known: usize,
    pub total: usize,
}

pub struct Need {
    pub dimension: Dimension,
    pub amount: f32,
    pub source: NeedSource,
    pub rationale: String, // código + parámetros, no texto libre inventado
}
```

La UI puede mostrar `value_estimate` solo cuando `value_high - value_low ≤ 0.20`; si no, muestra el
intervalo. Esto evita barras de progreso falsamente precisas.

> En v0.5 la salida implementada se llama `TeamAnalysis` (por equipo) y `DraftAnalysis` (ambos
> equipos). Incluye coberturas, daño, curva, rango, CC, movilidad, necesidades, condiciones de
> victoria, sinergias derivadas, señales y confianza; `DraftAnalysis` añade la lectura de líneas y
> las ventajas de equipo. La app expone `analysis_current`, que analiza el draft activo con el
> conjunto de perfiles activo. Las sinergias curadas y los matchups observados siguen pendientes y
> la UI no los simula.

## 11. Ejemplo sintético (fixture, no datos reales)

Fixture `SYNTH_ENGAGE_DPS` con tres campeones ficticios:

| Campeón | engage | frontline | dps | magic share | range | gold_dep |
| --- | --- | --- | --- | --- | --- | --- |
| `SYNTH_TANK` | 0.90 | 0.90 | 0.20 | 0.00 | 0.10 | 0.35 |
| `SYNTH_MAGE` | 0.20 | 0.10 | 0.60 | 0.95 | 0.75 | 0.80 |
| `SYNTH_CARRY` | 0.00 | 0.00 | 0.90 | 0.05 | 0.85 | 0.90 |

Agregación de `engage` con roles todos con `proficiency = 1.0`:

```text
e = [0.90, 0.20, 0.00]
A_engage = 0.90 + 0.30*0.20 + 0.10*0.00 = 0.96
```

`frontline`:

```text
e = [0.90, 0.10, 0.00]
A_frontline = 0.90 + 0.03 = 0.93
```

`dps`:

```text
e = [0.90, 0.60, 0.20]
A_dps = 0.90 + 0.18 + 0.02 = 1.10 → 1.00
```

Reparto de daño (pesos de oro 1.0 para los tres, por simplicidad del fixture):

```text
P_magic = (0.00 + 0.95 + 0.05) / 3 = 0.333
P_physical = 0.667
```

No hay necesidad de daño mágico por el umbral 0.30, pero sí de frontline si faltara. Si el rival
tiene `poke = 0.65` y este equipo tiene `engage = 0.96`, la regla `ENEMY_POKE` no añade engage (ya
está cubierto), pero puede añadir `sustain` si está por debajo de 0.35. Ese es el comportamiento
esperado: las reglas miran la cobertura actual, no solo la amenaza.

Este fixture se usa en tests de monotonía, no como dato de campeón.

## 12. Calibración y validación

Sin calibración, los parámetros son hipótesis. El plan es:

1. **Tests de propiedades** (obligatorios en Fase 2):
   - añadir cobertura de un arquetipo nunca la reduce;
   - un campeón con más intensidad no puede reducir la cobertura;
   - un desconocido no puede convertir una carencia confirmada en fortaleza;
   - los shares de daño suman 1;
   - ninguna salida contiene `NaN` ni valores fuera de rango;
   - la asignación de roles es óptima frente a fuerza bruta en fixtures pequeños.
2. **Revisión experta:** 20–30 composiciones etiquetadas a mano en las dimensiones principales
   (0–5), con dos revisores y desacuerdos documentados. Sirven para detectar errores groseros, no
   para "probar" el modelo.
3. **Sensibilidad:** variar cada peso ±20 % y comprobar que el top-5 de recomendaciones no cambia
   caóticamente. Si un peso domina, se corrige el diseño, no se esconde.
4. **Backtesting** (Fase 5): predecir el siguiente pick/ban en drafts profesionales y comparar con
   baselines. Se mide top-1/top-3/MRR, no winrate de la recomendación.
5. **Documentar límites:** el motor no conoce ejecución, forma del día, draft de series, ni
   adaptación en vivo. No se presenta como oráculo.

## 13. Parámetros y versionado

Todo lo numérico del análisis saturante vive en `AnalysisParams`. Las tablas de sinergias y matchups
viven en `analysis::interactions` y se versionan aparte con `INTERACTIONS_VERSION`, porque cambian
con las reglas y no con los pesos de agregación.

```rust
pub struct AnalysisParams {
    pub version: u32,
    pub saturation_weights: [f32; 3],   // [1.0, 0.30, 0.10]
    pub role_gold_weights: [f32; 5],
    pub floors: BTreeMap<Dimension, f32>,
    pub thresholds: BTreeMap<ThresholdId, f32>,
    pub rule_weights: BTreeMap<RuleId, f32>,
    pub prior_strength: PriorStrength,  // soloq / pro
}
```

Cambiar cualquier valor exige:
- incrementar `version` o `INTERACTIONS_VERSION` según corresponda;
- añadir/actualizar tests;
- registrar el cambio en el historial de perfiles;
- volver a ejecutar la suite de calibración.

La UI de configuración expone solo un subconjunto seguro (prioridad de dimensiones, fuente de
estadísticas, umbrales de confianza). Los pesos internos de calibración no se editan a mano sin
modo avanzado.
