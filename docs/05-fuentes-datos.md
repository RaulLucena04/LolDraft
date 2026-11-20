# 05 — Fuentes de datos e integración

> Estado: diseño e investigación, con el primer entregable de la Fase 4 ya implementado: esquema
> validado de estadísticas observadas, importación CSV/JSON y shrinkage en el motor. Sigue sin haber
> ningún scraper ni descarga automática. Las conclusiones de licencias deben revisarse antes de
> publicar datos de terceros.

## 1. Resumen de la investigación

| Fuente | Estado verificado | Uso previsto | Licencia / condición |
| --- | --- | --- | --- |
| **Data Dragon (Riot)** | Activa. `versions.json` devuelve hasta `16.22.1`; `champion.json` incluye 173 campeones | Identidad, assets, stats base | Activos de Riot; uso bajo la política de terceros. Sin API key |
| **Leaguepedia / Liquipedia** | API MediaWiki 1.43.10 con Cargo operativa | Drafts profesionales históricos | Contenido de la wiki; habitualmente CC BY-SA. Requiere atribución y respetar límites |
| **Lolalytics** | No hay API pública. `a3.lolalytics.com` se declara API privada y prohíbe su uso por terceros | Estadísticas de solo queue (si se obtiene permiso) | Copyright de LoLalytics Limited. **No se integra scraping por defecto** |
| **Riot API oficial** | Disponible con clave y rate limits | Verificación y estadísticas agregadas propias | Política de terceros de Riot; clave personal |
| **Oracle's Elixir** | Dataset comunitario de partidas profesionales | Validación cruzada de drafts/resultados | Revisar licencia y atribución antes de distribuir |
| **CommunityDragon** | Repositorio de assets | Assets alternativos | Propiedad de Riot; uso permitido con condiciones |

Durante la investigación, la API de Leaguepedia devolvió `ratelimited` tras varias consultas
seguidas. Eso confirma que el importador debe ser conservador: una petición cada segundo como
máximo, backoff exponencial, caché y sincronización incremental. No se debe diseñar asumiendo una
API rápida y tolerante.

## 2. Arquitectura de fuentes

```rust
#[async_trait]
pub trait DataSource {
    fn id(&self) -> &'static str;
    fn kind(&self) -> SourceKind;
    async fn fetch(&self, ctx: &FetchContext) -> Result<ImportBatch, SourceError>;
}

pub struct ImportBatch {
    pub source: String,
    pub kind: SourceKind,
    pub patch: Option<String>,
    pub fetched_at: i64,
    pub license: Option<String>,
    pub checksum: String,
    pub champions: Vec<ChampionIdentity>,
    pub champion_stats: Vec<ChampionStatRow>,
    pub matchup_stats: Vec<MatchupStatRow>,
    pub pro_games: Vec<ProGame>,
    pub warnings: Vec<ImportWarning>,
}
```

Reglas:

- `sources` **no escribe** en SQLite. Devuelve un lote normalizado.
- La app valida el lote y `data` lo persiste de forma transaccional.
- Cada adaptador declara su `SourceKind`, su licencia y si está habilitado por defecto.
- Los adaptadores no comparten parsers específicos; sí comparten cliente HTTP, caché y utilidades de
  normalización (nombres, parches, fechas).
- Un fallo de una fuente no bloquea el simulador ni el análisis: se sigue trabajando con el último
  dataset válido y se muestra el estado.

## 3. Data Dragon

### 3.1 Endpoints

```text
GET https://ddragon.leagueoflegends.com/api/versions.json
GET https://ddragon.leagueoflegends.com/cdn/{version}/data/{locale}/champion.json
GET https://ddragon.leagueoflegends.com/cdn/{version}/data/{locale}/champion/{id}.json
GET https://ddragon.leagueoflegends.com/cdn/{version}/img/champion/{id}.png
GET https://ddragon.leagueoflegends.com/cdn/{version}/img/champion/loading/{id}_0.jpg
```

- `locale` configurable (`es_ES`, `en_US`). Los nombres visibles se localizan; la clave es
  `riot_key`.
- Se elige la última versión estable de `versions.json`. Las versiones PBE o con sufijo raro no se
  usan por defecto.
- `champion.json` trae `id`, `key`, `name`, `title`, `tags`, `info`, `stats`. **No** trae
  arquetipos estratégicos: los tags de Riot (`Tank`, `Mage`, `Assassin`...) son demasiado gruesos
  para el motor y no se traducen automáticamente a intensidades.
- `champion/{id}.json` se usa solo si más adelante se necesita información de habilidades para
  proponer perfiles. No se usa para inventar arquetipos.

### 3.2 Sincronización

1. Descargar `versions.json` (cacheado con ETag).
2. Comparar con `champions.ddragon_version`.
3. Si cambia, descargar `champion.json` y hacer *upsert* por `riot_key`.
4. Detectar campeones nuevos: se insertan con perfil estratégico **desconocido**. La UI los muestra
   como "perfil pendiente", no con datos inventados.
5. Detectar campeones retirados/renombrados: no se borran si hay drafts que los referencian; se
   marcan como `retired` o se mantiene el slug anterior.
6. Descargar imágenes solo de los campeones que falten en caché; guardar con la versión en la ruta.

### 3.3 Límites

Data Dragon es estática y cacheable. Aun así: timeout, reintentos con backoff, `User-Agent`
identificable, caché en disco y comprobación de checksum. No se descarga en cada arranque.

## 4. Leaguepedia (drafts profesionales)

### 4.1 API

MediaWiki + Cargo:

```text
GET https://lol.fandom.com/api.php
  ?action=cargoquery
  &tables=ScoreboardGames
  &fields=GameId,Tournament,Team1,Team2,Winner,Patch,DateTime_UTC
  &where=...
  &order_by=DateTime_UTC DESC
  &limit=500
  &format=json
```

Buenas prácticas:

- `User-Agent` con nombre y contacto; no usar el de un navegador.
- `maxlag=5` cuando proceda.
- Una petición por segundo como máximo; backoff exponencial con jitter ante `ratelimited`.
- Cachear respuestas y sincronizar por torneo/fecha, no redescargar todo.
- Respetar los términos de Fandom/Liquipedia y la licencia del contenido.

### 4.2 Tablas a inspeccionar en implementación

La estructura exacta de Cargo debe verificarse contra la API antes de escribir el parser. Candidatas:

- `ScoreboardGames`: identidad del juego, torneo, equipos, ganador, parche, fecha, baneos.
- `ScoreboardPlayers`: jugador, equipo, campeón, rol, `GameId`.
- `MatchSchedule`: calendario y metadatos del enfrentamiento.
- `Tournaments` / `Teams`: normalización de competición y equipos.

**Regla:** si la fuente no aporta el orden de picks, se guarda `seq = NULL` y `order_known = false`.
No se inventa un orden "razonable". El backtesting de orden solo usa partidas con orden conocido.

### 4.3 Normalización

- Campeón: mapear nombre de Leaguepedia a `riot_key` con una tabla de alias versionada. Los alias
  cambian (nombres antiguos, abreviaturas); un fallo de mapeo va a cuarentena, no se descarta en
  silencio.
- Equipos: tabla de alias; no se fuerza una entidad canónica si la fuente no la da.
- Parche: normalizar `16.22` / `16.22.1` a `major.minor`; guardar el valor crudo.
- Resultado: `winner_side` solo si la fuente lo permite; si no, `NULL`.
- Deduplicación: `(dataset_id, source_game_id)` único. Reimportar el mismo torneo no duplica.
- Remakes, partidas incompletas y datos contradictorios: se marcan y se excluyen del análisis.

### 4.4 Licencia y atribución

El contenido de Leaguepedia suele estar bajo CC BY-SA. Antes de distribuir un dataset derivado:

- mostrar atribución visible en la app y en la documentación;
- conservar la licencia del dataset;
- no mezclar datos de Lolalytics (copyright) en el mismo archivo sin permiso;
- permitir exportar el dataset con su atribución.

La app puede usar los datos localmente con más flexibilidad, pero la versión distribuible debe
respetar la licencia. Esto se revisará antes de publicar.

## 5. Lolalytics

### 5.1 Hallazgos

- No existe una API pública documentada.
- El host `a3.lolalytics.com` se presenta como API privada para visitantes del sitio y declara que
  su contenido tiene copyright y **no puede ser usado por terceros**.
- `robots.txt` permite el rastreo general con `Content-Signal: use=reference`, pero eso no equivale
  a una licencia de uso de los datos. El aviso de copyright es más específico.
- Los scrapers comunitarios existen, pero dependen de HTML/JSON no documentado y se rompen con
  cambios del sitio. Además, copiar sus datos puede violar las condiciones del proveedor.

### 5.2 Decisión

**No se implementa un scraper automático de Lolalytics en la versión por defecto.** Sería frágil y
podría violar las condiciones del proveedor. El proyecto no debe basar su valor en datos que no
puede usar legítimamente.

Opciones, en orden de preferencia:

1. **Permiso escrito / licencia de datos.** Contactar con Lolalytics y acordar uso, límites y
   atribución. Si se concede, se implementa un adaptador oficial con caché y rate limit.
2. **Importación manual por el usuario.** El usuario exporta o copia los datos que necesita y la app
   los importa desde CSV/JSON. La app no descarga ni redistribuye nada. Útil para uso personal.
3. **Adaptador opcional de comunidad, deshabilitado por defecto.** Solo si el usuario lo activa
   conscientemente y asume las condiciones. No se empaqueta como fuente principal ni se distribuye
   con datos.
4. **API oficial de Riot.** Calcular estadísticas propias desde `match-v5` para solo queue. Es
   legal con clave, pero costoso: rate limits, volumen y necesidad de filtrar por elo. No sustituye
   a Lolalytics en el corto plazo.

Mientras no haya permiso, la app funciona con:

- datos profesionales de Leaguepedia;
- perfiles estratégicos curados propios;
- estadísticas importadas manualmente o desde una fuente con licencia clara;
- el motor de análisis, que no necesita winrates para funcionar.

### 5.3 Si se concede permiso

Campos a importar:

- winrate, pickrate, banrate por campeón/rol/parche/tier/región;
- matchups por campeón/oponente/rol;
- sinergias de dúo si están disponibles;
- tamaño de muestra y fecha de extracción.

Requisitos:

- `User-Agent` identificable y rate limit conservador;
- caché con TTL por parche;
- no redistribuir los datos crudos;
- mostrar "Fuente: Lolalytics" y la fecha;
- guardar `dataset.license` y `dataset.metadata_json` con las condiciones;
- permitir desactivar la fuente.

## 6. Riot API oficial (opcional)

- Requiere clave; se guarda en el llavero del sistema, nunca en SQLite ni en la UI.
- `match-v5` permite obtener partidas; para agregados por campeón/rol hay que muestrear y filtrar
  por elo, lo que introduce sesgo y coste.
- Útil para verificar datos importados y para un modo "solo queue propio", no para construir un
  clon de Lolalytics.
- Respetar rate limits y la política de aplicaciones de terceros de Riot. No se almacenan datos
  personales innecesarios.

## 7. Caché

### 7.1 Capas

| Capa | Ubicación | Contenido | Invalidación |
| --- | --- | --- | --- |
| HTTP | `cache/http/` | cuerpo + cabeceras + ETag + fecha | TTL por fuente / ETag |
| Imágenes | `cache/images/` | assets de Data Dragon | versión de DDragon |
| Datasets | SQLite | filas normalizadas | nuevo dataset / borrado manual |
| Perfiles | SQLite | `profile_set` | nueva versión |

### 7.2 Comportamiento

- Modo offline: la app arranca y funciona con el último dataset; muestra "sin conexión" y la fecha.
- `If-None-Match` / `If-Modified-Since` para no redescargar.
- TTL: DDragon 6 h; Leaguepedia 24 h; estadísticas 12 h o hasta cambio de parche.
- Límite de tamaño de caché (por ejemplo, 500 MB) con LRU; nunca se borra el dataset activo.
- Un `429` respeta `Retry-After` y activa un circuito que evita martillear la fuente.
- Los errores de parseo se registran con el fragmento mínimo necesario, sin guardar HTML completo
  más allá de TTL.

## 8. Actualización de datos

Flujo:

```text
Configuración → "Actualizar"
  → data_sync(sourceId)
  → descarga con caché
  → parseo a ImportBatch
  → validación (rangos, claves, duplicados, muestra)
  → staging
  → transacción de promoción
  → data_status actualizado
```

La UI muestra:

- última actualización por fuente;
- parche de los datos;
- número de filas y advertencias;
- licencia y atribución;
- botón de reintento;
- opción de importar archivo local.

No se actualiza en segundo plano sin que el usuario pueda verlo. Una actualización automática
opcional se añade en Fase 4, con ventana de mantenimiento y sin bloquear el simulador.

## 9. Calidad de datos

Validaciones mínimas:

- `winrate ∈ [0,1]`, `games ≥ 0`, `pickrate/banrate ∈ [0,1]`;
- `champion_key` existe en `champions`;
- `opponent_key ≠ champion_key`;
- parche con formato reconocible;
- shares de daño suman ~1;
- matchup con `games` por encima del mínimo para mostrarse como estadística;
- sin duplicados dentro del dataset;
- checksum del lote.

Cuando una fila falla, se descarta o se envía a cuarentena con motivo. El resumen de importación
dice cuántas se descartaron y por qué. Nunca se completa un hueco con un promedio inventado.

### 9.1 Formato de importación de estadísticas (Fase 4)

El primer entregable de la Fase 4 está en `loldraft-engine::stats`. Es código puro: valida el
documento y calcula la señal. La persistencia (`loldraft-data::stats`, tabla `observed_stats`) y la
pantalla de importación ya existen.

Documento JSON (autodescriptivo, estricto):

```json
{
  "schema": "1",
  "id": "manual-soloq-16.22",
  "version": "1",
  "source": "Exportación manual",
  "license": "Uso personal; no redistribuir",
  "queue": "soloq",
  "patch": "16.22",
  "rows": [
    { "champion": 64, "role": "jungle", "games": 1200, "wins": 620, "pick_rate": 0.08, "ban_rate": 0.12 }
  ]
}
```

CSV equivalente (cabecera obligatoria; orden libre; `pick_rate` y `ban_rate` opcionales):

```csv
champion,role,games,wins,pick_rate,ban_rate
64,jungle,1200,620,0.08,0.12
103,mid,900,470,,
```

Reglas:

- `queue` es `soloq` o `pro`; nunca se mezclan en la misma cifra.
- `patch` debe ser `major.minor` o `major.minor.patch`.
- `games > 0`, `wins <= games`, tasas en `0..1`, sin pares `(campeón, rol)` duplicados.
- JSON: un campo desconocido o una fila inválida rechazan el dataset completo.
- CSV: las filas inválidas se descartan con su motivo y la importación devuelve la lista; si no queda
  ninguna fila válida, falla. No se admiten comillas: el formato es simple y comprobable.
- El winrate crudo se muestra, pero el motor solo usa el **ajustado**:
  `(wins + 200 × 0.5) / (games + 200)`. Con 200 partidas la confianza es 0.5; con 50, 0.2.
- `power` no usa el winrate aislado: usa el percentil del winrate ajustado **dentro del mismo rol**
  entre filas con al menos 200 partidas. Así no se compara jungla con support ni se inventa un
  baseline externo. Si no hay dos filas comparables en el rol, no hay señal.
- Cada fila conserva su `n`; por debajo de 200 partidas se muestra pero no puntúa.

Desde la pantalla de inicio se puede importar un JSON o un CSV. El dataset se guarda entero en
SQLite y se activa **para su cola**: activar SoloQ no toca el dataset profesional. La lista muestra
id, versión, parche, filas, fuente, licencia y antigüedad, y permite activar o borrar. Si un CSV deja
filas inválidas, la interfaz muestra la lista de cuarentena con línea y motivo.

El dataset activo alimenta la señal `power` de las recomendaciones, pero solo en su cola: el
simulador competitivo lee el activo de `pro` y el modo SoloQ el de `soloq`. Cada respuesta incluye la
procedencia del dataset (id, versión, cola, parche, fuente, licencia, filas y mínimo de partidas) y
la UI la muestra bajo la recomendación. Si falta el dataset de esa cola, `power` se marca ausente y
los pesos se renormalizan; nunca se cruza el de la otra cola.

Lo que no hace todavía: descargar ni convertir datos de Lolalytics. La importación es siempre de un
fichero local que elige el usuario.

### 9.2 Formato de importación de builds y runas (Fase 4)

La build y las runas salen del mismo principio que las estadísticas: un documento local que declara
su fuente y su licencia. El motor (`loldraft-engine::builds`) elige la build; no la inventa.

```json
{
  "schema": "1",
  "id": "manual-soloq-16.22",
  "version": "1",
  "source": "Exportación manual",
  "license": "Uso personal; no redistribuir",
  "queue": "soloq",
  "patch": "16.22",
  "rows": [
    {
      "champion": 64,
      "role": "jungle",
      "name": "Conqueror bruiser",
      "starter_items": ["Doran's Blade", "Health Potion"],
      "core_items": ["Eclipse", "Sundered Sky", "Black Cleaver"],
      "situational_items": ["Death's Dance"],
      "runes": {
        "primary_path": "Precision",
        "keystone": "Conqueror",
        "primary_runes": ["Triumph", "Legend: Alacrity", "Last Stand"],
        "secondary_path": "Inspiration",
        "secondary_runes": ["Magical Footwear", "Cosmic Insight"],
        "shards": ["Adaptive Force", "Adaptive Force", "Health Scaling"]
      },
      "games": 1200,
      "wins": 620
    }
  ]
}
```

Reglas:

- `queue` es `soloq` o `pro`; activar un dataset de una cola no toca el de la otra.
- `patch` con el mismo formato que las estadísticas.
- `core_items` no puede estar vacío: una build sin núcleo no es una build.
- `runes.primary_path` y `runes.secondary_path` deben ser distintas; `primary_runes` admite como
  máximo 4, `secondary_runes` 3 y `shards` 3. Los fragmentos pueden repetirse (dos veces «Adaptive
  Force» es legítimo); los objetos de una misma lista no.
- No hay `(campeón, rol, nombre)` repetidos. `wins <= games`.
- `games = 0` describe una **build curada sin muestra**: se muestra como tal y nunca se presenta como
  winrate observado.
- El documento es estricto: un campo desconocido o una fila inválida rechazan el dataset completo.

Selección para un campeón (y opcionalmente un rol):

1. entre las filas con al menos 200 partidas, la de mejor winrate ajustado con el mismo shrinkage
   que las estadísticas;
2. si ninguna llega al mínimo, la fila con más partidas (se etiqueta «muestra insuficiente»);
3. si ninguna tiene partidas, la primera del fichero (se etiqueta «build curada»).

Los empates conservan el orden del fichero: manda el dataset, no una heurística oculta. La interfaz
muestra junto a la build la muestra, el winrate ajustado, la confianza, la procedencia (id, versión,
cola, parche, fuente, licencia, filas y mínimo) y el motivo de la elección. Si no hay dataset activo
para la cola, o el dataset no tiene fila para ese campeón, se dice explícitamente; no se usa la build
de otra cola ni se rellena con una genérica.

`data/samples/builds-soloq.example.json` es un ejemplo de estructura para probar la importación. Sus
filas son de demostración (`games: 0`), no estadísticas: sustitúyelo por datos propios.

## 10. Atribución en la aplicación

Pantalla "Acerca de / Datos":

- Riot Games: aviso de no afiliación y propiedad de activos.
- Data Dragon: versión y fuente.
- Leaguepedia: atribución y licencia del dataset.
- Lolalytics: solo si se usa con permiso; texto de atribución acordado.
- Oracle's Elixir: atribución si se usa.
- Fecha de última actualización y checksum del dataset activo.
- Botón para exportar la lista de fuentes y licencias.

## 11. Riesgos técnicos de las fuentes

| Riesgo | Impacto | Mitigación |
| --- | --- | --- |
| Lolalytics sin API/permiso | Alto | Abstracción de fuentes; modo manual; no scraping por defecto |
| Rate limit de Leaguepedia | Medio | 1 req/s, backoff, caché, sync incremental |
| Cambios de esquema Cargo | Medio | Parser versionado, validación, tests con fixtures |
| Nombres de campeones cambian | Bajo | Clave numérica + alias versionados |
| Parches mezclados | Alto | Dataset por parche; fallback visible, nunca silencioso |
| Muestras pro pequeñas | Alto | Shrinkage, confianza, no concluir con n bajo |
| Licencias de datasets | Alto | Registro de licencia por dataset; revisión antes de publicar |
| Scraper frágil | Medio | No depender de HTML; solo APIs/archivos estructurados |

## 12. Lo que no se hará

- No se scrapea Lolalytics en la build distribuible sin permiso.
- No se inventan estadísticas para completar campeones.
- No se mezclan solo queue y profesional en una sola cifra sin etiqueta.
- No se usa un winrate sin `n`, parche y fuente.
- No se descarga ni ejecuta código de una fuente.
- No se guardan credenciales en el frontend.
