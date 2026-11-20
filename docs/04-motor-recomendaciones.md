# 04 — Motor de recomendaciones

> Estado: diseño. Implementación en Fase 3, después de que el análisis de composiciones esté
> calibrado y testeado.

## 1. Contrato

Dado un estado de draft, devolver para la **acción actual**:

- los mejores picks o baneos disponibles, ordenados;
- una puntuación de ajuste estratégico (0–100), no una probabilidad de victoria;
- la contribución exacta de cada criterio a esa puntuación;
- motivos, fortalezas aportadas, carencias cubiertas, sinergias, matchups y riesgos;
- alternativas y nivel de confianza;
- una explicación que se pueda auditar contra los números.

El motor es determinista: mismo estado, mismos datos y mismos pesos producen el mismo ranking. No
usa aleatoriedad ni un LLM.

## 2. Universo de candidatos

Un campeón es candidato si:

1. no está baneado ni picado por ninguno de los dos equipos;
2. tiene al menos un rol libre viable para el equipo que actúa;
3. su perfil estratégico no está completamente ausente (si lo está, puede aparecer solo en una
   sección "sin datos", nunca con una puntuación inventada).

Para cada candidato se evalúan todas las asignaciones de rol compatibles (máximo 120). Se conserva
la mejor asignación y se registra su ventaja sobre la segunda. Si la ventaja es pequeña, la
recomendación muestra "rol ambiguo" y baja la confianza de la asignación.

No se descartan los flex picks por ocupar un rol ya cubierto: se evalúa la recomposición completa
del equipo, no la suma de roles individuales.

## 3. Features

Todas las features se normalizan a `[0,1]` **antes** de ponderar. Una feature sin datos no vale
0.5: se marca como ausente y se renormalizan los pesos de las restantes. Esto evita que la falta de
datos parezca neutralidad.

### 3.1 `need` — cubre carencias reales

Se calcula como ganancia marginal del análisis de composición:

```text
before_d = cobertura del arquetipo d en el equipo actual
after_d  = cobertura del arquetipo d al añadir el candidato en el rol asignado

need_fit = Σ_d need_d * (after_d - before_d)
           ───────────────────────────────────
                    Σ_d need_d
```

Propiedades:

- Si el equipo no tiene carencias (`Σ need_d = 0`), la feature se marca ausente y el peso se
  reparte entre las demás.
- Solo cuenta la ganancia marginal; un campeón que ya está cubierto no suma por repetir.
- Usa las cotas conservadoras del análisis: si el dato es desconocido, la ganancia se calcula con la
  cota inferior y la confianza baja.
- Es explicable: se puede mostrar exactamente qué carencia sube y cuánto.

### 3.2 `answer` — neutraliza amenazas del rival

Mide cuánto reduce la severidad de las amenazas enemigas, no cuánto cubre una carencia propia:

```text
answer = Σ_r threat_severity_r * reduction_r(c) / Σ_r threat_severity_r
```

Ejemplos:

- rival con `engage` alto: valora `disengage`, `peel`, `frontline`, `zone_control`;
- rival con `poke` alto: valora `engage`, `sustain`, `waveclear`;
- rival con `frontline` alto: valora `dps`, `anti_tank`;
- rival con `assassin`/`burst`: valora `peel`, `frontline`, `hard_cc`;
- rival con `splitpush`: valora `catch`, `waveclear`, `mobility` de rotación.

Cada reducción se calcula con las reglas del motor de composiciones, de modo que pick y análisis
comparten la misma lógica. No hay una segunda tabla de "counters" desconectada.

### 3.3 `synergy` — encaja con los aliados

```text
synergy = Σ_{i∈team} w_i * normalize(edge(c,i)) / Σ w_i
```

- `edge ∈ [-1,1]`; se normaliza a `[0,1]`.
- Las sinergias curadas pesan más que las derivadas (doc 03).
- Una anti-sinergia fuerte reduce la feature y además genera riesgo.
- Si no hay datos de sinergia, la feature se marca ausente; no se asume 0.5.

### 3.4 `counter` — matchup contra el rival

Combina:

```text
counter = 0.55 * lane_edge + 0.45 * team_edge
```

- `lane_edge`: matchup del campeón frente al rival del rol asignado, con *shrinkage* y confianza.
- `team_edge`: mejora del enfrentamiento global (doc 03, sección 8.3).
- Si no hay estadísticas, se usa el matchup por arquetipos con un descuento de confianza.
- Se muestra el `n` y la fuente. Un 52 % con 30 partidas no pesa como un 52 % con 20 000.

### 3.5 `power` — fuerza observada en el parche

No es winrate crudo. Es el percentil del winrate **ajustado** dentro del mismo rol:

```text
wr_adj  = (wins + 200 × 0.5) / (games + 200)   // shrinkage
conf    = games / (games + 200)                // confianza de la fila
power   = percentil(wr_adj) entre las filas del mismo rol con games ≥ 200
```

- El prior (200 partidas al 50 %) evita que un 3–0 parezca imbatible y que un 1–4 parezca
  catastrófico. El winrate crudo se muestra como dato, nunca como señal.
- El percentil se calcula **solo dentro del mismo rol**: comparar un support con un mid mezclaría
  bases distintas. No se usa un baseline externo ni un promedio inventado; el percentil sale del
  propio dataset. Con menos de dos filas elegibles en el rol no hay señal.
- Los empates comparten la media de sus posiciones.
- La confianza que acompaña a la feature es la de la fila (`games / (games + 200)`), no una constante.
- Se separan `soloq` y `pro` de forma estricta: cada modo lee solo el dataset activo de su cola.
  Si no hay dataset activo para esa cola, `power` queda ausente; nunca se sustituye por el de la otra.
- `pick_rate` y `ban_rate` se aceptan en el esquema como datos opcionales, pero **todavía no** entran
  en el cálculo. Cuando se usen, se documentará la fórmula de presencia.
- `power` tiene un peso deliberadamente bajo en picks tardíos. **Nunca** puede decidir por sí solo.
- La respuesta lleva la procedencia (`StatsProvenance`: id, versión, cola, parche, fuente, licencia,
  filas y mínimo de partidas) para que la UI no muestre un percentil anónimo.

### 3.6 `flex` — flexibilidad e información revelada

```text
flex = min(1, (roles_viables - 1) / 2)
```

- 1 rol → 0; 2 roles → 0.5; 3 o más → 1.
- Se pondera por la calidad de los roles secundarios (`proficiency`) y por si el rol alternativo
  sigue libre.
- En picks tempranos vale más porque oculta información y preserva respuestas.
- En últimos picks vale 0: ya no hay nada que ocultar.
- Si hay datos de distribución de roles, se añade una penalización por "pick que revela rol"
  (por ejemplo, un campeón que el 95 % de las veces es support).

### 3.7 `balance` — mejora el reparto de recursos y daño

```text
balance = 1 - penalty
```

Penalizaciones:

- exceso de un tipo de daño (`> 0.70`) y el candidato añade el mismo tipo;
- exceso de campeones `gold_dependency` altos;
- exceso de `scaling` si el equipo ya no tiene presión temprana;
- falta de frontline si el candidato es otro cuerpo frágil;
- duplicación de una función que ya tiene dos especialistas (por ejemplo, tercer `poke`).

`balance` no es "equilibrio perfecto": es reducción de un desequilibrio concreto. La UI muestra qué
penalización se evita o se agrava.

### 3.8 `safety` — seguridad del pick en su contexto

Usa `lane_safety`, `mobility`, `range` y la vulnerabilidad del rol:

- picks a ciegas valoran campeones difíciles de castigar;
- picks con información del rival valoran menos la seguridad y más el counter;
- un campeón con `execution_difficulty` alta no se penaliza por ser difícil, pero sí se avisa del
  riesgo de ejecución si el equipo tiene poco CC o poca protección.

En Fase 1 puede quedarse en una versión simple basada en `lane_safety`; se amplía cuando haya datos.

## 4. Puntuación

```text
score_raw = Σ_f w_f * feature_f          // Σ w_f = 1; features ausentes renormalizan
risk      = Σ_k risk_k                   // 0..1, capado
score     = clamp(score_raw - 0.15 * risk, 0, 1)
display   = round(100 * score)
```

- `risk` es multiplicativo solo a través de su coeficiente; no se esconde dentro de una feature.
- Cada `risk_k` tiene su propio motivo (`REDUNDANT_DAMAGE`, `RESOURCE_CONFLICT`, ...).
- `display` es un índice de ajuste estratégico. La UI debe decirlo explícitamente; no es "55 % de
  ganar".
- Para ordenar se usa:

```text
ranking = score - 0.10 * (1 - confidence)
```

La puntuación mostrada y la de ordenación pueden diferir; la UI muestra ambas cuando la confianza
es baja, con la etiqueta "datos limitados".

### 4.1 Pesos por fase

Perfiles iniciales, configurables y versionados:

| Feature | Early pick | Mid pick | Late pick |
| --- | ---: | ---: | ---: |
| `need` | 0.12 | 0.26 | 0.24 |
| `synergy` | 0.10 | 0.24 | 0.10 |
| `counter` | 0.03 | 0.10 | 0.28 |
| `power` | 0.18 | 0.12 | 0.08 |
| `flex` | 0.22 | 0.10 | 0.00 |
| `balance` | 0.05 | 0.06 | 0.10 |
| `answer` | 0.05 | 0.07 | 0.15 |
| `safety` | 0.25 | 0.05 | 0.05 |
| **Suma** | **1.00** | **1.00** | **1.00** |

Lectura:

- **Early pick:** seguridad a ciegas + flexibilidad; el counter todavía no existe porque no hay
  información. `power` importa, pero no domina.
- **Mid pick:** sinergia y necesidades; empieza a definirse la identidad.
- **Late pick:** counter y respuesta a amenazas; flexibilidad ya no aporta.

Modificadores de contexto (se aplican y se renormaliza):

| Contexto | Efecto |
| --- | --- |
| Primer pick del draft (blue P1) | `flex +0.10`, `safety +0.10`, `counter −0.05` |
| Último pick del draft (red P5) | `counter +0.10`, `answer +0.05`, `flex −0.10` |
| Rival con 3+ picks revelados | `counter +0.05`, `answer +0.05` |
| Rival con 0–1 picks revelados | `flex +0.05`, `safety +0.05` |
| Tu equipo ya tiene 2+ carries de recursos | `balance +0.05`, `power −0.02` |

Los modificadores se validan para que ningún peso quede negativo y la suma siga siendo 1.

## 5. Contexto del draft

```rust
pub enum DraftPhase {
    EarlyBan,   // primeros 6 baneos
    EarlyPick,  // primeros 6 picks
    LateBan,    // últimos 4 baneos
    LatePick,   // últimos 4 picks
}

pub struct DraftContext {
    pub phase: DraftPhase,
    pub action_index: u8,       // 1..20
    pub side: Side,
    pub picks_remaining: u8,
    pub enemy_revealed: u8,
    pub is_first_pick: bool,
    pub is_last_pick: bool,
    pub format_id: String,
}
```

El contexto no cambia las features: cambia sus pesos. Así una recomendación sigue siendo auditable
("tenía mejor counter, pero en primer pick pesa más la seguridad").

Casos especiales:

- **Early ban:** se recomienda banear, no pickear. Se usa el motor de baneos (sección 7).
- **Late ban:** el objetivo es proteger el plan propio y negar el counter que el rival puede
  completar.
- **Red side:** tiene la última respuesta; el motor sube `counter`/`answer` cuando quedan pocos
  picks. No se asume que el rojo siempre tenga ventaja: se mide con el draft concreto.
- **Flex ambigua:** si dos campeones pueden ocupar el mismo rol, el motor evalúa la asignación
  óptima y muestra la alternativa. La UI permite fijar el rol manualmente.

## 6. Diversidad y anti-repetición

Dos problemas distintos:

1. **Repetición entre drafts:** el mismo campeón meta aparece siempre. No se resuelve con
   aleatoriedad; se resuelve con features de necesidad y sinergia y con un peso de `power` bajo.
2. **Repetición dentro del top:** cinco ADC no es una lista útil. Se aplica diversificación tipo
   MMR sobre el top N:

```text
for each slot:
  adjusted_i = ranking_i - λ * max_{j∈selected} overlap(i,j)
  overlap = 0.5 * same_role + 0.5 * cosine(archetype_vectors)
```

- `λ = 0.12` inicial.
- Se garantiza como máximo 2 candidatos por rol en el top 5, salvo que no haya alternativas.
- No se altera la puntuación mostrada; solo el orden de la lista de alternativas. La primera
  recomendación es siempre la de mayor `ranking`.
- La lista debe incluir al menos un candidato de un rol distinto si existe y su score no es
  catastróficamente inferior.

Esto también ayuda a que el usuario explore respuestas, no solo el pick "obvio".

## 7. Recomendaciones de baneo

Un ban no es "el campeón con mejor winrate". Es una acción sobre el plan del rival. Se evalúa desde
la perspectiva del rival:

```text
opp_gain(c) = recommend(opponent_team + c, perspective=opponent).score_gain
deny(c) = 0.40 * opp_need_gain(c)
        + 0.20 * opp_synergy(c)
        + 0.15 * opp_power(c)
        + 0.10 * opp_flex(c)
        + 0.15 * own_threat(c)
```

Donde:

- `opp_need_gain`: cuánto mejora la composición rival con ese campeón.
- `opp_synergy`: sinergia con los picks rivales ya revelados.
- `opp_power`: percentil observado del campeón en el rol rival, con peso bajo. Usa el mismo dataset
  de la cola y la misma regla de muestra mínima que `power`; sin fila suficiente, se marca ausente.
- `opp_flex`: amenaza de flex / información que niega.
- `own_threat`: cuánto castiga ese campeón al plan propio (counter directo a nuestros picks).

Se resta el interés propio:

```text
final_deny = deny - 0.25 * own_pick_value(c)
```

Así no se banea un campeón que el equipo quiere picar, salvo que el rival lo vaya a aprovechar
mucho más.

Fases:

- **Primeros baneos:** niegan flexibilidad y poder general; poca información del rival.
- **Últimos baneos:** niegan el counter concreto que el rival puede completar y protegen el pick
  propio planeado.

Cada baneo incluye "por qué": "el rival necesita engage y este campeón es su mejor fuente
disponible; además es flex en dos roles".

## 8. Explicaciones

### 8.1 Contribuciones

```rust
pub struct Contribution {
    pub feature: FeatureId,
    pub raw: f32,          // valor de la feature
    pub normalized: f32,   // 0..1
    pub weight: f32,
    pub contribution: f32, // weight * normalized
}

pub struct RiskContribution {
    pub kind: RiskId,
    pub amount: f32,
    pub contribution: f32, // negativo
}
```

Invariante testeada:

```text
sum(contributions) + sum(risk_contributions) = score
```

Si la suma no cuadra, el test falla. La UI puede mostrar un desglose tipo waterfall y el usuario
puede comprobar por qué un campeón está arriba.

### 8.2 Motivos

Los motivos son códigos con parámetros, no frases generadas libremente:

| Código | Ejemplo de plantilla |
| --- | --- |
| `FILLS_GAP` | "Aporta {dimension}: {before} → {after} (necesidad {need})" |
| `AMPLIFIES_STRENGTH` | "Potencia {dimension} de {before} a {after}" |
| `SYNERGY` | "Sinergia {tag} con {champion} ({score})" |
| `ANTI_SYNERGY` | "Solapa mal con {champion} en {tag}" |
| `COUNTERS` | "Buen matchup contra {champion} en {role} ({edge}, n={games})" |
| `ANSWERS_THREAT` | "Responde a {threat} enemigo con {response}" |
| `FLEX_VALUE` | "Puede jugar {roles}; conserva información en {phase}" |
| `META_POWER` | "Presencia alta en {queue} ({presence}, n={games})" |
| `DAMAGE_BALANCE` | "Corrige el reparto de daño: {before} → {after}" |
| `REDUNDANT_DAMAGE` | "Añade {type} a un equipo ya cargado de {type}" |
| `RESOURCE_CONFLICT` | "Compite por recursos con {allies}" |
| `LOW_SAMPLE` | "Muestra pequeña: {games} partidas en {source}" |
| `UNKNOWN_PROFILE` | "Perfil estratégico incompleto; confianza reducida" |
| `ROLE_AMBIGUOUS` | "Puede jugar {roles}; el rol óptimo no es claro" |

La UI traduce las plantillas (i18n) y muestra los parámetros. No hay texto que afirme algo que no
esté en las contribuciones.

### 8.3 Salida completa

```rust
pub struct RecommendationSet {
    pub session_id: String,
    pub phase: DraftPhase,
    pub engine_version: String,
    pub profile_set_id: String,
    pub dataset_id: Option<String>,
    pub picks: Vec<Recommendation>,
    pub bans: Vec<BanRecommendation>,
    pub generated_at: i64,
}

pub struct Recommendation {
    pub champion: ChampionRef,
    pub role: Role,
    pub score: f32,             // 0..100 mostrado
    pub ranking_score: f32,
    pub confidence: f32,
    pub contributions: Vec<Contribution>,
    pub risks: Vec<RiskContribution>,
    pub reasons: Vec<Reason>,
    pub fills: Vec<GapFix>,
    pub synergies: Vec<SynergyNote>,
    pub matchups: Vec<MatchupNote>,
    pub drawbacks: Vec<Reason>,
}
```

`picks` contiene las 5–10 mejores opciones; no se anidan alternativas por campeón. La UI muestra la
lista y, al seleccionar una, el detalle.

## 9. Ejemplo sintético de decisión (fixture)

Equipo ficticio con carencias: `engage 0.30`, `frontline 0.40`, daño mágico 15 %. Dos candidatos
ficticios:

| Feature | Candidato A | Candidato B |
| --- | ---: | ---: |
| `need` | 0.85 | 0.20 |
| `synergy` | 0.65 | 0.35 |
| `counter` | 0.40 | 0.55 |
| `power` | 0.45 | 0.90 |
| `flex` | 0.50 | 0.20 |
| `balance` | 0.80 | 0.20 |
| `answer` | 0.70 | 0.25 |
| `safety` | 0.40 | 0.30 |
| `risk` | 0.05 | 0.10 |

Con pesos de pick tardío:

```text
A = .24*.85 + .10*.65 + .28*.40 + .08*.45 + 0*.50 + .10*.80 + .15*.70 + .05*.40
  = .204 + .065 + .112 + .036 + 0 + .080 + .105 + .020 = .622
A_final = .622 - .15*.05 = .6145  → 61

B = .24*.20 + .10*.35 + .28*.55 + .08*.90 + 0 + .10*.20 + .15*.25 + .05*.30
  = .048 + .035 + .154 + .072 + 0 + .020 + .0375 + .015 = .3815
B_final = .3815 - .15*.10 = .3665  → 37
```

B tiene mucho mejor winrate (`power 0.90`), pero solo aporta el 8 % del peso y no cubre las
carencias. A gana. Este es el comportamiento que el proyecto debe garantizar y testear.

## 10. Confianza y ausencia de datos

- La confianza final combina: cobertura de perfiles, confianza de estadísticas, confianza de
  asignación de rol y cobertura de sinergias.
- Si una feature no tiene datos, se excluye y se renormaliza; la UI indica qué faltó.
- Si la confianza es `< 0.45`, la recomendación se muestra con etiqueta "datos limitados" y no se
  presenta como concluyente.
- Si no hay ningún candidato con perfil mínimo, el motor devuelve `NoData` con una explicación, no
  una lista vacía sin contexto.
- Los datos inferidos nunca se mezclan visualmente con los observados sin una marca.

## 11. Tests obligatorios

1. **Determinismo:** misma entrada → misma salida byte a byte (salvo timestamp).
2. **Suma de contribuciones:** coincide con la puntuación dentro de `1e-5`.
3. **Monotonía de necesidad:** si aumenta una carencia, el candidato que la cubre sube o iguala.
4. **Dominancia:** si A ≥ B en todas las features y > en una, A no puede quedar por debajo.
5. **No winrate-only:** un fixture con B muy por encima en `power` y A cubriendo carencias debe
   hacer ganar a A con los pesos por defecto.
6. **Fase:** el mismo estado en early/late produce pesos y explicaciones distintas.
7. **Flex:** un campeón con tres roles viables sube en early y no en late.
8. **Diversidad:** el top 5 no tiene más de dos candidatos del mismo rol si hay alternativas.
9. **Baneos:** un campeón que el rival aprovecharía mucho sube como ban; uno que nuestro equipo
   quiere picar baja.
10. **Datos ausentes:** sin perfiles, no hay puntuaciones inventadas; se devuelve `NoData`.
11. **Sin NaN/infinitos** con cualquier combinación de ceros, desconocidos y muestras pequeñas.
12. **Sensibilidad:** variar un peso ±20 % no debe cambiar el top-1 de forma arbitraria en los
    fixtures de referencia; si cambia, el test documenta por qué es aceptable.

## 12. Calibración

Los pesos iniciales son una hipótesis razonada, no una verdad. Se calibran en Fase 5 con:

- **Predicción del siguiente pick/ban** en drafts profesionales (top-1, top-3, MRR).
- **Baselines:** uniforme, mayor presencia, mayor winrate, y el propio motor sin una feature
  (ablación).
- **Holdout temporal:** entrenar/ajustar con torneos antiguos y evaluar con torneos posteriores;
  nunca dividir aleatoriamente partidas del mismo torneo.
- **Sensibilidad:** mapa de calor de pesos vs métricas. Si el rendimiento depende de un peso
  afinado al tercer decimal, el modelo es frágil.
- **Revisión cualitativa:** 30 recomendaciones revisadas por un analista, marcando errores graves.

El resultado se documenta como "el motor acierta el siguiente pick en el top-3 un X % frente a un
Y % del baseline", no como "el motor predice el draft".

## 13. Qué no hace el motor

- No estima probabilidad de victoria.
- No conoce la forma de los jugadores, comunicaciones, preparación de serie ni adaptación en vivo.
- No inventa un perfil que falta.
- No usa el winrate como atajo.
- No cambia sus explicaciones después de calcular: primero se calculan contribuciones, luego se
  verbalizan.
